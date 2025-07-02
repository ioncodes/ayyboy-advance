#!/usr/bin/env python3
"""
parse_gba_hash.py

Reads a MAME GBA software-list XML file and outputs a CSV with:
  - crc32             (from the first <rom> element)
  - savetype_code     (numeric code per mapping)
  - rtc               ("1" if any feature value mentions RTC, else "0")
  - description       (from <description> text)

Mapping:
  0: No Backup
  1: Eeprom4k
  2: Eeprom64k
  3: Sram
  4: Flash 512k
  5: Flash 1M

Usage:
  python parse_gba_hash.py <input_xml> <output_csv>
"""

import xml.etree.ElementTree as ET
import csv
import sys

# Order to check features for savetype, lower-case
SAVETYPE_FEATURE_ORDER = ["slot", "u1", "u2", "savetype", "save"]


def map_savetype_to_code(savetype_str):
    """
    Map a savetype text to a numeric code:
      0: No Backup
      1: Eeprom4k
      2: Eeprom64k
      3: Sram
      4: Flash 512k
      5: Flash 1M
    """
    s = savetype_str.lower()
    if not s:
        return 0
    if 'eeprom' in s:
        return 2 if '64' in s else 1
    if 'sram' in s:
        return 3
    if 'flash' in s:
        return 4 if '512' in s or '512k' in s else 5
    return 0


def parse_gba_xml_to_csv(xml_file_path, csv_file_path):
    # Load XML
    with open(xml_file_path, "r", encoding="utf-8") as f:
        root = ET.fromstring(f.read())

    seen_crcs = set()
    # Write CSV
    with open(csv_file_path, "w", newline="", encoding="utf-8") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow(["crc32", "savetype_code", "rtc", "description"])

        for software in root.findall('.//software'):
            # CRC32
            crc = ''
            part = software.find('part')
            if part is not None:
                for dataarea in part.findall('dataarea'):
                    rom = dataarea.find('rom')
                    if rom is not None and rom.get('crc'):
                        crc = rom.get('crc')
                        break

            # Skip duplicates
            if not crc or crc in seen_crcs:
                continue
            seen_crcs.add(crc)

            # Description
            desc_elem = software.find('description')
            description = desc_elem.text.strip() if desc_elem is not None else ''

            # RTC flag
            has_rtc = 0
            # Gather features
            feature_map = {}
            if part is not None:
                for feat in part.findall('feature'):
                    name = feat.get('name', '').lower()
                    val = feat.get('value', '').strip()
                    if not name or not val:
                        continue
                    feature_map[name] = val
                    if 'rtc' in val.lower():
                        has_rtc = 1

            # Determine savetype string, prioritizing slot
            savetype_str = ''
            # 1. software attribute
            if software.get('savetype'):
                savetype_str = software.get('savetype').strip()
            else:
                # 2. feature order
                for key in SAVETYPE_FEATURE_ORDER:
                    if key in feature_map:
                        savetype_str = feature_map[key]
                        break

            savetype_code = map_savetype_to_code(savetype_str)

            writer.writerow([crc, savetype_code, has_rtc, description])


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input_xml> <output_csv>")
        sys.exit(1)

    input_xml, output_csv = sys.argv[1], sys.argv[2]
    parse_gba_xml_to_csv(input_xml, output_csv)
