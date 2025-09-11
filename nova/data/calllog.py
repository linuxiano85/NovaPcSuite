"""Call log export functionality."""

import csv
import json
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

from ..adb.content_providers import ContentProvider
from ..adb.device import ADBDevice
from ..util.logging import get_logger
from ..util.paths import ensure_directory
from ..util.timeutil import timestamp_to_iso

logger = get_logger(__name__)


class CallLogExporter:
    """Exports call log from Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.content_provider = ContentProvider(device)
    
    def export_call_log(
        self,
        output_dir: Path,
        formats: List[str] = ["json", "csv"],
        limit: Optional[int] = None
    ) -> Dict[str, str]:
        """Export call log in specified formats."""
        
        logger.info("Starting call log export...")
        
        # Get call log data
        call_log = self.content_provider.get_call_log(limit=limit)
        
        if not call_log:
            logger.warning("No call log entries found or access denied")
            return {}
        
        logger.info(f"Found {len(call_log)} call log entries to export")
        
        ensure_directory(output_dir)
        
        # Process call log entries
        processed_entries = self._process_call_log_entries(call_log)
        
        exported_files = {}
        
        # Export in requested formats
        if "json" in formats:
            json_file = self._export_json(processed_entries, output_dir)
            if json_file:
                exported_files["json"] = str(json_file)
        
        if "csv" in formats:
            csv_file = self._export_csv(processed_entries, output_dir)
            if csv_file:
                exported_files["csv"] = str(csv_file)
        
        return exported_files
    
    def _process_call_log_entries(self, call_log: List[Dict[str, str]]) -> List[Dict[str, any]]:
        """Process and clean call log entries."""
        processed = []
        
        for entry in call_log:
            processed_entry = {
                "id": entry.get("_id", ""),
                "number": entry.get("number", ""),
                "name": entry.get("name", ""),
                "call_type": entry.get("call_type", "unknown"),
                "duration": self._format_duration(entry.get("duration", "0")),
                "duration_seconds": int(entry.get("duration", "0")) if entry.get("duration", "").isdigit() else 0,
                "date": self._format_date(entry.get("date", "")),
                "date_timestamp": int(entry.get("date", "0")) if entry.get("date", "").isdigit() else 0,
                "cached_number_type": entry.get("cached_number_type", ""),
                "cached_number_label": entry.get("cached_number_label", ""),
            }
            
            processed.append(processed_entry)
        
        return processed
    
    def _format_duration(self, duration_str: str) -> str:
        """Format call duration in human readable format."""
        try:
            duration_seconds = int(duration_str)
            
            if duration_seconds == 0:
                return "0:00"
            
            hours = duration_seconds // 3600
            minutes = (duration_seconds % 3600) // 60
            seconds = duration_seconds % 60
            
            if hours > 0:
                return f"{hours}:{minutes:02d}:{seconds:02d}"
            else:
                return f"{minutes}:{seconds:02d}"
                
        except (ValueError, TypeError):
            return "0:00"
    
    def _format_date(self, date_str: str) -> str:
        """Format call date in ISO format."""
        try:
            timestamp = int(date_str)
            return timestamp_to_iso(timestamp / 1000)  # Android timestamps are in milliseconds
        except (ValueError, TypeError):
            return ""
    
    def _export_json(self, call_log: List[Dict[str, any]], output_dir: Path) -> Optional[Path]:
        """Export call log to JSON format."""
        try:
            json_file = output_dir / "call_log.json"
            
            export_data = {
                "export_timestamp": datetime.now().isoformat(),
                "total_entries": len(call_log),
                "device_serial": self.device.serial,
                "entries": call_log
            }
            
            with open(json_file, "w", encoding="utf-8") as f:
                json.dump(export_data, f, indent=2, ensure_ascii=False)
            
            logger.info(f"Exported {len(call_log)} call log entries to {json_file}")
            return json_file
            
        except Exception as e:
            logger.error(f"Failed to export call log to JSON: {e}")
            return None
    
    def _export_csv(self, call_log: List[Dict[str, any]], output_dir: Path) -> Optional[Path]:
        """Export call log to CSV format."""
        try:
            csv_file = output_dir / "call_log.csv"
            
            if not call_log:
                # Create empty CSV with headers
                with open(csv_file, "w", newline="", encoding="utf-8") as f:
                    writer = csv.writer(f)
                    writer.writerow(["id", "number", "name", "call_type", "duration", "date"])
                return csv_file
            
            # Use first entry to determine fieldnames
            fieldnames = list(call_log[0].keys())
            
            with open(csv_file, "w", newline="", encoding="utf-8") as f:
                writer = csv.DictWriter(f, fieldnames=fieldnames)
                writer.writeheader()
                
                for entry in call_log:
                    # Convert all values to strings
                    csv_entry = {k: str(v) for k, v in entry.items()}
                    writer.writerow(csv_entry)
            
            logger.info(f"Exported {len(call_log)} call log entries to {csv_file}")
            return csv_file
            
        except Exception as e:
            logger.error(f"Failed to export call log to CSV: {e}")
            return None
    
    def get_call_log_summary(self) -> Dict[str, any]:
        """Get summary information about call log."""
        try:
            call_log = self.content_provider.get_call_log(limit=1000)  # Sample for stats
            
            if not call_log:
                return {"total": 0, "has_access": False}
            
            # Calculate statistics
            call_types = {}
            total_duration = 0
            
            for entry in call_log:
                call_type = entry.get("call_type", "unknown")
                call_types[call_type] = call_types.get(call_type, 0) + 1
                
                duration = entry.get("duration", "0")
                if duration.isdigit():
                    total_duration += int(duration)
            
            return {
                "total": len(call_log),
                "call_types": call_types,
                "total_duration_seconds": total_duration,
                "total_duration_formatted": self._format_duration(str(total_duration)),
                "has_access": True
            }
            
        except Exception as e:
            logger.error(f"Failed to get call log summary: {e}")
            return {"total": 0, "has_access": False, "error": str(e)}
    
    def test_access(self) -> bool:
        """Test if call log can be accessed."""
        try:
            access_results = self.content_provider.test_content_provider_access()
            return access_results.get("call_log", False)
        except Exception:
            return False