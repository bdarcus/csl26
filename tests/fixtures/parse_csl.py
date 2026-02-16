#!/usr/bin/env python3
import sys
import json
import re

def parse_csl_test(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    sections = {}
    current_section = None
    buffer = []

    for line in content.splitlines():
        start_match = re.match(r">>=====\s*([A-Z-]+)\s*=====>>", line)
        end_match = re.match(r"<<=====\s*([A-Z-]+)\s*=====<<", line)

        if start_match:
            current_section = start_match.group(1)
            buffer = []
        elif end_match:
            if current_section:
                sections[current_section] = "\n".join(buffer).strip()
                current_section = None
        elif current_section:
            buffer.append(line)

    result = {}
    
    if "MODE" in sections:
        result["mode"] = sections["MODE"]
    
    if "DESCRIPTION" in sections:
        result["description"] = sections["DESCRIPTION"]
        
    if "RESULT" in sections:
        result["result"] = sections["RESULT"]
        
    if "CSL" in sections:
        result["csl"] = sections["CSL"]
        
    if "INPUT" in sections:
        try:
            result["input_items"] = json.loads(sections["INPUT"])
        except json.JSONDecodeError:
             # Sometimes INPUT is not valid JSON but Python eval-able (e.g. tuples), but CSL test suite is usually valid JSON
             # Check if we need to fix anything? 
             pass

    if "CITATION-ITEMS" in sections:
        try:
            result["citation_items"] = json.loads(sections["CITATION-ITEMS"])
        except json.JSONDecodeError:
            pass

    return result

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: parse_csl.py <filepath>")
        sys.exit(1)
        
    filepath = sys.argv[1]
    data = parse_csl_test(filepath)
    print(json.dumps(data, indent=2))
