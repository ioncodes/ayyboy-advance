import sys
import glob
import re
import csv
import concurrent.futures
from tqdm import tqdm
from zipfile import ZipFile 

SAVE_TYPE_PATTERNS = [
    (re.compile(rb"EEPROM_V\d\d\d"), "EEPROM (512 bytes or 8KB)", 0),
    (re.compile(rb"SRAM_V\d\d\d"), "SRAM", 1),
    (re.compile(rb"FLASH_V\d\d\d"), "Flash 64KB", 2),
    (re.compile(rb"FLASH512_V\d\d\d"), "Flash 64KB", 2),
    (re.compile(rb"FLASH1M_V\d\d\d"), "Flash 128KB", 3),
]

def extract_gba_rom(zip_path):
    with ZipFile(zip_path, "r") as zip_file:
        for file_info in zip_file.infolist():
            if file_info.filename.endswith(".gba"):
                with zip_file.open(file_info.filename) as gba_file:
                    return gba_file.read()
    print(f"Warning: No GBA ROM found in {zip_path}")
    return None

def detect_save_type(rom):
    for pattern, _, save_type  in SAVE_TYPE_PATTERNS:
        if pattern.search(rom):
            return save_type
    return None

def extract_game_code(rom):
    code_bytes = rom[0xAC:0xB0]
    return code_bytes.decode("ascii").strip()

def extract_pretty_game_title(path):
    filename = path.split("\\")[-1]
    return filename.split("(")[0].strip()

GAMECODES = []
def process_zip(zip_path):
    rom = extract_gba_rom(zip_path)
    if rom is None:
        return None
    save_type = detect_save_type(rom)
    if save_type is None:
        return None
    game_code = extract_game_code(rom)
    if game_code in GAMECODES:
        return None
    GAMECODES.append(game_code)
    pretty_title = extract_pretty_game_title(zip_path)
    return game_code, save_type, pretty_title

def main():
    import_path = sys.argv[1]
    export_path = sys.argv[2]

    csvfile = open(export_path, "w", newline="")
    writer = csv.writer(csvfile)

    zip_paths = glob.glob(f"{import_path}/*.zip")
    with concurrent.futures.ThreadPoolExecutor() as executor:
        results = executor.map(process_zip, zip_paths)
        for result in tqdm(results, total=len(zip_paths), desc="Processing ZIPs"):
            if result:
                writer.writerow(list(result))


if __name__ == "__main__":
    main()