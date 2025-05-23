import sys
import re


def detect_format(line):
    """Detect which format the trace line is in."""
    if line.startswith('[debug'):
        return 'format2'  # ayyboy_advance debug (verbose_debug feature) format
    else:
        return 'format1'  # Mesen2 trace format


def parse_line(line):
    """Parse a trace line to extract address and register values."""
    format_type = detect_format(line)
    address = "unknown"
    registers = {}
    
    if format_type == 'format1':
        # Format 1: "08000000 B $080000C0 R0:00000000 R1:00000000..."
        # Extract address from the beginning
        addr_match = re.match(r'^([0-9a-fA-F]{8})', line)
        if addr_match:
            address = "0x" + addr_match.group(1).lower()
            
        # Extract register values - handle both uppercase and lowercase
        reg_matches = re.findall(r'[Rr](\d+):([0-9a-fA-F]+)', line)
        for reg_num, reg_val in reg_matches:
            if int(reg_num) <= 14:  # r0 through r14
                registers[f'r{reg_num.lower()}'] = int(reg_val, 16)
    
    else:  # format2
        # Format 2: "[debug ayyboy_advance::arm7tdmi::cpu] 08000000: b +184 [r0=00000000..."
        # Extract address
        addr_match = re.search(r'([0-9a-fA-F]{8}):[^[]*\[', line)
        if addr_match:
            address = "0x" + addr_match.group(1).lower()
            
        # Extract normal registers
        reg_matches = re.findall(r'[Rr](\d+)=([0-9a-fA-F]+)', line)
        for reg_num, reg_val in reg_matches:
            if int(reg_num) <= 14:  # r0 through r14
                registers[f'r{reg_num.lower()}'] = int(reg_val, 16)
                
        # Extract special registers (sp, lr)
        sp_match = re.search(r'sp=([0-9a-fA-F]+)', line, re.IGNORECASE)
        if sp_match:
            registers['r13'] = int(sp_match.group(1), 16)
            
        lr_match = re.search(r'lr=([0-9a-fA-F]+)', line, re.IGNORECASE)
        if lr_match:
            registers['r14'] = int(lr_match.group(1), 16)
    
    if address == "unknown":
        raise ValueError(f"Address not found in line: {line.strip()}")
            
    return address, registers


def compare_traces(file1, file2):
    """Compare two trace files line by line."""
    with open(file1, 'r') as f1, open(file2, 'r') as f2:
        line_num = 0
        lines1 = f1.readlines()
        lines2 = f2.readlines()
        
        i1 = 0
        i2 = 0
        
        while i1 < len(lines1) and i2 < len(lines2):
            line_num += 1
            line1 = lines1[i1]
            line2 = lines2[i2]
            
            # Check for BLL/BLH pair in format1
            skip_next_line1 = False
            if detect_format(line1) == 'format1' and 'BLL' in line1 and i1+1 < len(lines1) and 'BLH' in lines1[i1+1]:
                # We'll handle this as a single instruction, so next iteration should skip a line
                skip_next_line1 = True
                # For the comparison, we'll use the BLL line but take note that it's a paired instruction
                # print(f"Note: BLL/BLH instruction pair at line {line_num}")
            
            # Parse both lines
            addr1, regs1 = parse_line(line1)
            addr2, regs2 = parse_line(line2)
            
            failed = False

            # Check if addresses match (case-insensitive)
            if addr1.lower() != addr2.lower():
                print(f"Address mismatch: {addr1} vs {addr2}")
                failed = True
            
            # Check registers
            for i in range(13):  # r0 through r12
                reg_name = f'r{i}'
                if reg_name in regs1 and reg_name in regs2:
                    if regs1[reg_name] != regs2[reg_name]:
                        print(f"{reg_name} mismatch at {addr1}: "
                              f"0x{regs1[reg_name]:08x} vs 0x{regs2[reg_name]:08x}")
                        failed = True

            if failed:
                print(f"Line {line_num} mismatch detected")
                return False
            
            # Move to next line(s)
            i1 += 2 if skip_next_line1 else 1
            i2 += 1
    return True


def main():
    if len(sys.argv) != 3:
        print("Usage: python compare_trace.py trace_file1 trace_file2")
        sys.exit(1)
    
    file1 = sys.argv[1]
    file2 = sys.argv[2]
    
    try:
        equivalent = compare_traces(file1, file2)
        if equivalent:
            print("Traces are equivalent")
        else:
            print("Traces are not equivalent")
    except Exception as e:
        print(f"Error during comparison: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
