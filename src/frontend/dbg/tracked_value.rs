#[derive(Default, Copy, Clone)]
pub struct TrackedValue<T> {
    value: T,
    changed: bool,
}

impl<T> std::fmt::Display for TrackedValue<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T> std::fmt::Debug for TrackedValue<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl<T> TrackedValue<T>
where
    T: Copy + PartialEq,
{
    pub fn set(&mut self, value: T) {
        if value != self.value {
            self.value = value;
            self.changed = true;
        } else {
            self.changed = false;
        }
    }

    pub fn get(&self) -> T {
        self.value
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }
}
