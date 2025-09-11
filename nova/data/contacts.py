"""Contacts export functionality."""

import csv
from pathlib import Path
from typing import Dict, List, Optional

from ..adb.content_providers import ContentProvider
from ..adb.device import ADBDevice
from ..util.logging import get_logger
from ..util.paths import ensure_directory, safe_filename

logger = get_logger(__name__)


class ContactsExporter:
    """Exports contacts from Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.content_provider = ContentProvider(device)
    
    def export_contacts(
        self,
        output_dir: Path,
        formats: List[str] = ["vcf", "csv"]
    ) -> Dict[str, str]:
        """Export contacts in specified formats."""
        
        logger.info("Starting contacts export...")
        
        # Get contacts data
        contacts = self.content_provider.get_contacts()
        
        if not contacts:
            logger.warning("No contacts found or access denied")
            return {}
        
        logger.info(f"Found {len(contacts)} contacts to export")
        
        ensure_directory(output_dir)
        
        exported_files = {}
        
        # Export in requested formats
        if "vcf" in formats:
            vcf_file = self._export_vcf(contacts, output_dir)
            if vcf_file:
                exported_files["vcf"] = str(vcf_file)
        
        if "csv" in formats:
            csv_file = self._export_csv(contacts, output_dir)
            if csv_file:
                exported_files["csv"] = str(csv_file)
        
        return exported_files
    
    def _export_vcf(self, contacts: List[Dict[str, str]], output_dir: Path) -> Optional[Path]:
        """Export contacts to vCard format."""
        try:
            vcf_file = output_dir / "contacts.vcf"
            
            with open(vcf_file, "w", encoding="utf-8") as f:
                for contact in contacts:
                    vcard = self._create_vcard(contact)
                    f.write(vcard)
                    f.write("\n")
            
            logger.info(f"Exported {len(contacts)} contacts to {vcf_file}")
            return vcf_file
            
        except Exception as e:
            logger.error(f"Failed to export contacts to VCF: {e}")
            return None
    
    def _export_csv(self, contacts: List[Dict[str, str]], output_dir: Path) -> Optional[Path]:
        """Export contacts to CSV format."""
        try:
            csv_file = output_dir / "contacts.csv"
            
            # Determine all possible fields
            all_fields = set()
            for contact in contacts:
                all_fields.update(contact.keys())
            
            # Sort fields for consistent output
            fieldnames = sorted(all_fields)
            
            with open(csv_file, "w", newline="", encoding="utf-8") as f:
                writer = csv.DictWriter(f, fieldnames=fieldnames)
                writer.writeheader()
                
                for contact in contacts:
                    # Clean up list fields for CSV
                    clean_contact = {}
                    for key, value in contact.items():
                        if isinstance(value, list):
                            clean_contact[key] = "; ".join(str(v) for v in value)
                        else:
                            clean_contact[key] = str(value) if value else ""
                    
                    writer.writerow(clean_contact)
            
            logger.info(f"Exported {len(contacts)} contacts to {csv_file}")
            return csv_file
            
        except Exception as e:
            logger.error(f"Failed to export contacts to CSV: {e}")
            return None
    
    def _create_vcard(self, contact: Dict[str, str]) -> str:
        """Create a vCard entry for a contact."""
        vcard_lines = ["BEGIN:VCARD", "VERSION:3.0"]
        
        # Full name
        display_name = contact.get("display_name", "").strip()
        if display_name:
            # Basic name parsing (could be improved)
            name_parts = display_name.split()
            if len(name_parts) >= 2:
                first_name = name_parts[0]
                last_name = " ".join(name_parts[1:])
                vcard_lines.append(f"N:{last_name};{first_name};;;")
            else:
                vcard_lines.append(f"N:{display_name};;;;")
            
            vcard_lines.append(f"FN:{display_name}")
        
        # Phone numbers
        phone_numbers = contact.get("phone_numbers", [])
        if isinstance(phone_numbers, list):
            for phone in phone_numbers:
                if phone:
                    # Clean phone number
                    clean_phone = self._clean_phone_number(phone)
                    vcard_lines.append(f"TEL:{clean_phone}")
        
        # Email addresses
        email_addresses = contact.get("email_addresses", [])
        if isinstance(email_addresses, list):
            for email in email_addresses:
                if email:
                    vcard_lines.append(f"EMAIL:{email}")
        
        # Additional fields
        contact_id = contact.get("_id", "")
        if contact_id:
            vcard_lines.append(f"UID:{contact_id}")
        
        times_contacted = contact.get("times_contacted", "")
        if times_contacted:
            vcard_lines.append(f"X-TIMES-CONTACTED:{times_contacted}")
        
        vcard_lines.append("END:VCARD")
        
        return "\n".join(vcard_lines)
    
    def _clean_phone_number(self, phone: str) -> str:
        """Clean and format phone number."""
        # Remove common formatting characters
        clean = "".join(c for c in phone if c.isdigit() or c in ["+", "-", " ", "(", ")"])
        return clean.strip()
    
    def get_contacts_summary(self) -> Dict[str, any]:
        """Get summary information about contacts."""
        try:
            contacts = self.content_provider.get_contacts()
            
            if not contacts:
                return {"total": 0, "has_access": False}
            
            # Calculate statistics
            total_contacts = len(contacts)
            contacts_with_phone = sum(1 for c in contacts if c.get("phone_numbers"))
            contacts_with_email = sum(1 for c in contacts if c.get("email_addresses"))
            
            return {
                "total": total_contacts,
                "with_phone": contacts_with_phone,
                "with_email": contacts_with_email,
                "has_access": True
            }
            
        except Exception as e:
            logger.error(f"Failed to get contacts summary: {e}")
            return {"total": 0, "has_access": False, "error": str(e)}
    
    def test_access(self) -> bool:
        """Test if contacts can be accessed."""
        try:
            access_results = self.content_provider.test_content_provider_access()
            return access_results.get("contacts", False)
        except Exception:
            return False