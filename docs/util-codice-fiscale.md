# Italian Codice Fiscale Generator & Validator

This utility provides generation and validation of Italian tax codes (Codice Fiscale) according to the official algorithm.

## Algorithm Summary

The Italian Codice Fiscale is a 16-character alphanumeric code composed of:

1. **Surname code** (3 chars): First 3 consonants, then vowels if needed, pad with X
2. **Name code** (3 chars): First 3 consonants, but if ≥4 consonants exist, take 1st, 3rd, and 4th
3. **Birth date and sex** (5 chars): 
   - Year (2 digits): Last 2 digits of birth year
   - Month (1 char): Letter encoding (A=Jan, B=Feb, C=Mar, D=Apr, E=May, H=Jun, L=Jul, M=Aug, P=Sep, R=Oct, S=Nov, T=Dec)
   - Day (2 digits): Day of month, +40 for females
4. **Birthplace code** (4 chars): Official comune code
5. **Control character** (1 char): Calculated checksum using specific algorithm

## Features

- **Generation**: Create valid Codice Fiscale from personal data
- **Validation**: Verify control character correctness
- **Decoding**: Extract birth information from existing codes
- **Comune lookup**: Built-in index of major Italian cities

## Dataset Note

The current implementation includes a limited set of major Italian comuni for demonstration purposes. For production use, a comprehensive dataset of all Italian municipalities should be integrated.

## Legal Disclaimer

⚠️ **IMPORTANT**: This utility is provided for educational and legitimate administrative purposes only.

- **Privacy Compliance**: Ensure compliance with GDPR and Italian privacy laws
- **Data Protection**: Do not misuse personal data
- **Legitimate Use**: Only use for authorized administrative, educational, or software development purposes
- **No Warranty**: This software is provided as-is without warranty

Users are responsible for ensuring their use of this utility complies with all applicable laws and regulations regarding personal data processing.

## Command Examples

### Generate a Codice Fiscale

```bash
# Generate for Mario Rossi, male, born 1990-05-15 in Roma
nova-cli cf generate \
  --surname "Rossi" \
  --name "Mario" \
  --birth-date "1990-05-15" \
  --sex "M" \
  --comune "Roma"

# Output: RSSMRA90E15H501S
```

### Validate a Codice Fiscale

```bash
# Validate a codice fiscale
nova-cli cf validate "RSSMRA90E15H501S"

# Output: ✓ Codice Fiscale 'RSSMRA90E15H501S' is VALID
```

### Decode a Codice Fiscale

```bash
# Decode information from a codice fiscale
nova-cli cf decode "RSSMRA90E15H501S"

# Output:
# Decoded information from 'RSSMRA90E15H501S':
#   Birth Year: 2090
#   Birth Month: 5
#   Birth Day: 15
#   Sex: M
#   Birthplace Code: H501
```

### Generate with Direct Birthplace Code

```bash
# If you know the exact birthplace code
nova-cli cf generate \
  --surname "Verdi" \
  --name "Giuseppe" \
  --birth-date "1985-12-03" \
  --sex "M" \
  --birthplace-code "H501"
```

## Supported Comuni

The utility currently includes codes for these major cities:

- Roma (H501)
- Milano (F205)
- Napoli (F839)
- Torino (L219)
- Palermo (G273)
- Genova (D969)
- Bologna (A944)
- Firenze (D612)
- Bari (A662)
- Catania (C351)

## Integration with Contact Export

This utility is designed to support advanced contact export and normalization features in NovaPcSuite:

- **Data Enrichment**: Add valid tax codes to contact datasets before archival
- **Data Validation**: Verify existing tax codes in contact databases
- **Data Cleanup**: Standardize and correct malformed tax codes
- **Export Enhancement**: Enrich contact exports with computed tax codes for Italian contacts

The modular design allows easy integration with the plugin system for future contact management features.

## Technical Implementation

- **No External Dependencies**: Uses only standard Rust libraries plus chrono for date handling
- **Memory Efficient**: Lazy-loaded lookup tables
- **Error Handling**: Comprehensive error types with descriptive messages
- **Safe**: No panics on user input, all operations return Results
- **Tested**: Full test coverage of all algorithm components

## Future Enhancements

- Complete Italian municipalities dataset integration
- Reverse lookup (comune code to name)
- Historical municipality code handling
- Foreign country code support
- Batch processing capabilities