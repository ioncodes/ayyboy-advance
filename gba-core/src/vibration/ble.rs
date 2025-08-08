use btleplug::api::{Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tracing::*;

pub struct VibrationManager {
    runtime: tokio::runtime::Runtime,
    peripheral: Arc<Mutex<Option<Peripheral>>>,
    write_char: Arc<Mutex<Option<Characteristic>>>,
}

impl VibrationManager {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        let peripheral = Arc::new(Mutex::new(None));
        let write_char = Arc::new(Mutex::new(None));

        let manager = Self {
            runtime,
            peripheral,
            write_char,
        };

        // Try to connect to device immediately
        manager.connect_device();

        manager
    }

    fn connect_device(&self) {
        let peripheral = Arc::clone(&self.peripheral);
        let write_char = Arc::clone(&self.write_char);

        // Run connection task immediately and block until complete
        if let Err(e) = self
            .runtime
            .block_on(async move { Self::discover_and_connect(peripheral, write_char).await })
        {
            error!(target: "vibration", "Failed to connect to vibration device: {}", e);
        }
    }

    async fn discover_and_connect(
        peripheral: Arc<Mutex<Option<Peripheral>>>, write_char: Arc<Mutex<Option<Characteristic>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;

        if adapters.is_empty() {
            return Err("No Bluetooth adapters found".into());
        }

        let central = &adapters[0];
        central.start_scan(ScanFilter::default()).await?;

        // Scan for 5 seconds
        time::sleep(Duration::from_secs(5)).await;
        central.stop_scan().await?;

        let peripherals = central.peripherals().await?;

        // Lovense regex patterns
        let service_regex = Regex::new(r"^..300001-002.-4bd4-bbd5-a6920e4c5653").unwrap();
        let tx_regex = Regex::new(r"^..300002-002.-4bd4-bbd5-a6920e4c5653").unwrap();

        for p in peripherals {
            if let Ok(Some(properties)) = p.properties().await {
                if let Some(name) = &properties.local_name {
                    info!(target: "vibration", "Found device: {}", name);

                    let lovense_regex = Regex::new(r"(?i)^lvs-").unwrap();
                    if lovense_regex.is_match(name) {
                        info!(target: "vibration", "Found Lovense device: {}", name);

                        // Check if already connected
                        if p.is_connected().await.unwrap_or(false) {
                            info!(target: "vibration", "Device {} already connected", name);
                        } else {
                            info!(target: "vibration", "Attempting to connect to {}", name);
                            if let Err(e) = p.connect().await {
                                warn!(target: "vibration", "Failed to connect to {}: {}", name, e);
                                continue;
                            }

                            // Wait a moment for connection to stabilize
                            time::sleep(Duration::from_millis(500)).await;

                            // Verify connection
                            if !p.is_connected().await.unwrap_or(false) {
                                warn!(target: "vibration", "Connection to {} failed to establish", name);
                                continue;
                            }

                            info!(target: "vibration", "Successfully connected to {}", name);
                        }

                        info!(target: "vibration", "Discovering services for {}", name);
                        if let Err(e) = p.discover_services().await {
                            warn!(target: "vibration", "Failed to discover services for {}: {}", name, e);
                            continue;
                        }

                        let services = p.services();
                        info!(target: "vibration", "Found {} services for {}", services.len(), name);

                        // Find the specific Lovense service and TX characteristic
                        for service in services {
                            let service_uuid = service.uuid.to_string();
                            info!(target: "vibration", "Checking service: {}", service_uuid);

                            if service_regex.is_match(&service_uuid) {
                                info!(target: "vibration", "Found Lovense service: {}", service_uuid);

                                for characteristic in &service.characteristics {
                                    let char_uuid = characteristic.uuid.to_string();
                                    info!(target: "vibration", "Checking characteristic: {}", char_uuid);

                                    if tx_regex.is_match(&char_uuid) {
                                        info!(target: "vibration", "Found Lovense TX characteristic: {}", char_uuid);

                                        *peripheral.lock().await = Some(p.clone());
                                        *write_char.lock().await = Some(characteristic.clone());

                                        // Send stop command to ensure device is in known state
                                        let stop_cmd = "Vibrate:0;".as_bytes().to_vec();
                                        if let Err(e) =
                                            p.write(&characteristic, &stop_cmd, WriteType::WithoutResponse).await
                                        {
                                            warn!(target: "vibration", "Failed to send initial stop command: {}", e);
                                        } else {
                                            info!(target: "vibration", "Sent initial stop command to ensure device is off");
                                        }

                                        info!(target: "vibration", "Successfully connected to vibration device");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err("No compatible vibration device found".into())
    }

    pub fn stop_all_vibration(&self) {
        let peripheral = Arc::clone(&self.peripheral);
        let write_char = Arc::clone(&self.write_char);

        let runtime_handle = self.runtime.handle().clone();
        std::thread::spawn(move || {
            runtime_handle.block_on(async move {
                let peripheral_guard = peripheral.lock().await;
                let char_guard = write_char.lock().await;

                if let (Some(peripheral), Some(characteristic)) = (peripheral_guard.as_ref(), char_guard.as_ref()) {
                    let stop_cmd = "Vibrate:0;".as_bytes().to_vec();
                    if let Err(e) = peripheral
                        .write(characteristic, &stop_cmd, WriteType::WithoutResponse)
                        .await
                    {
                        warn!(target: "vibration", "Failed to send stop command during cleanup: {}", e);
                    } else {
                        info!(target: "vibration", "Sent stop command during cleanup");
                    }
                } else {
                    debug!(target: "vibration", "No device connected for cleanup");
                }
            });
        });
    }

    pub fn vibrate(&self, duration_ms: u64) {
        let peripheral1 = Arc::clone(&self.peripheral);
        let peripheral2 = Arc::clone(&self.peripheral);
        let write_char1 = Arc::clone(&self.write_char);
        let write_char2 = Arc::clone(&self.write_char);

        let runtime_handle1 = self.runtime.handle().clone();
        let runtime_handle2 = self.runtime.handle().clone();

        // Run vibration in a separate thread to avoid blocking the caller
        std::thread::spawn(move || {
            runtime_handle1.block_on(async move {
                let peripheral_guard = peripheral1.lock().await;
                let char_guard = write_char1.lock().await;

                if let (Some(peripheral), Some(characteristic)) = (peripheral_guard.as_ref(), char_guard.as_ref()) {
                    info!(target: "vibration", "Sending vibration command for {}ms", duration_ms);

                    // Start vibration
                    let start_cmd = "Vibrate:5;".as_bytes().to_vec();
                    if let Err(e) = peripheral
                        .write(characteristic, &start_cmd, WriteType::WithoutResponse)
                        .await
                    {
                        warn!(target: "vibration", "Failed to start vibration: {}", e);
                        return;
                    }

                    info!(target: "vibration", "Vibration started, waiting {}ms", duration_ms);
                } else {
                    warn!(target: "vibration", "No vibration device connected for vibrate call");
                }
            });

            std::thread::sleep(Duration::from_millis(duration_ms));

            runtime_handle2.block_on(async move {
                let peripheral_guard = peripheral2.lock().await;
                let char_guard = write_char2.lock().await;

                if let (Some(peripheral), Some(characteristic)) = (peripheral_guard.as_ref(), char_guard.as_ref()) {
                    info!(target: "vibration", "Stopping vibration after {}ms", duration_ms);

                    // Stop vibration
                    let stop_cmd = "Vibrate:0;".as_bytes().to_vec();
                    if let Err(e) = peripheral
                        .write(characteristic, &stop_cmd, WriteType::WithoutResponse)
                        .await
                    {
                        warn!(target: "vibration", "Failed to stop vibration: {}", e);
                        return;
                    }

                    info!(target: "vibration", "Vibration stopped");
                } else {
                    warn!(target: "vibration", "No vibration device connected for vibrate call");
                }
            });
        });
    }
}
