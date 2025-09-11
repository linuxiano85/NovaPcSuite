"""SMS export functionality."""

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


class SMSExporter:
    """Exports SMS messages from Android device."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.content_provider = ContentProvider(device)
    
    def export_sms(
        self,
        output_dir: Path,
        formats: List[str] = ["json", "csv"],
        limit: Optional[int] = None
    ) -> Dict[str, str]:
        """Export SMS messages in specified formats."""
        
        logger.info("Starting SMS export...")
        
        # Get SMS data
        sms_messages = self.content_provider.get_sms_messages(limit=limit)
        
        if not sms_messages:
            logger.warning("No SMS messages found or access denied")
            return {}
        
        logger.info(f"Found {len(sms_messages)} SMS messages to export")
        
        ensure_directory(output_dir)
        
        # Process SMS messages
        processed_messages = self._process_sms_messages(sms_messages)
        
        exported_files = {}
        
        # Export in requested formats
        if "json" in formats:
            json_file = self._export_json(processed_messages, output_dir)
            if json_file:
                exported_files["json"] = str(json_file)
        
        if "csv" in formats:
            csv_file = self._export_csv(processed_messages, output_dir)
            if csv_file:
                exported_files["csv"] = str(csv_file)
        
        return exported_files
    
    def _process_sms_messages(self, sms_messages: List[Dict[str, str]]) -> List[Dict[str, any]]:
        """Process and clean SMS messages."""
        processed = []
        
        for message in sms_messages:
            processed_message = {
                "id": message.get("_id", ""),
                "thread_id": message.get("thread_id", ""),
                "address": message.get("address", ""),
                "body": message.get("body", ""),
                "message_type": message.get("message_type", "unknown"),
                "date": self._format_date(message.get("date", "")),
                "date_timestamp": int(message.get("date", "0")) if message.get("date", "").isdigit() else 0,
                "date_sent": self._format_date(message.get("date_sent", "")),
                "date_sent_timestamp": int(message.get("date_sent", "0")) if message.get("date_sent", "").isdigit() else 0,
                "is_read": message.get("is_read", False),
                "read_status": "read" if message.get("is_read", False) else "unread",
            }
            
            processed.append(processed_message)
        
        return processed
    
    def _format_date(self, date_str: str) -> str:
        """Format SMS date in ISO format."""
        try:
            timestamp = int(date_str)
            return timestamp_to_iso(timestamp / 1000)  # Android timestamps are in milliseconds
        except (ValueError, TypeError):
            return ""
    
    def _export_json(self, sms_messages: List[Dict[str, any]], output_dir: Path) -> Optional[Path]:
        """Export SMS messages to JSON format."""
        try:
            json_file = output_dir / "sms.json"
            
            export_data = {
                "export_timestamp": datetime.now().isoformat(),
                "total_messages": len(sms_messages),
                "device_serial": self.device.serial,
                "messages": sms_messages
            }
            
            with open(json_file, "w", encoding="utf-8") as f:
                json.dump(export_data, f, indent=2, ensure_ascii=False)
            
            logger.info(f"Exported {len(sms_messages)} SMS messages to {json_file}")
            return json_file
            
        except Exception as e:
            logger.error(f"Failed to export SMS to JSON: {e}")
            return None
    
    def _export_csv(self, sms_messages: List[Dict[str, any]], output_dir: Path) -> Optional[Path]:
        """Export SMS messages to CSV format."""
        try:
            csv_file = output_dir / "sms.csv"
            
            if not sms_messages:
                # Create empty CSV with headers
                with open(csv_file, "w", newline="", encoding="utf-8") as f:
                    writer = csv.writer(f)
                    writer.writerow(["id", "address", "body", "message_type", "date", "read_status"])
                return csv_file
            
            # Use first message to determine fieldnames
            fieldnames = list(sms_messages[0].keys())
            
            with open(csv_file, "w", newline="", encoding="utf-8") as f:
                writer = csv.DictWriter(f, fieldnames=fieldnames)
                writer.writeheader()
                
                for message in sms_messages:
                    # Convert all values to strings and handle text content
                    csv_message = {}
                    for k, v in message.items():
                        if k == "body":
                            # Clean up message body for CSV
                            body = str(v) if v else ""
                            # Replace newlines with space
                            body = body.replace("\n", " ").replace("\r", " ")
                            csv_message[k] = body
                        else:
                            csv_message[k] = str(v)
                    
                    writer.writerow(csv_message)
            
            logger.info(f"Exported {len(sms_messages)} SMS messages to {csv_file}")
            return csv_file
            
        except Exception as e:
            logger.error(f"Failed to export SMS to CSV: {e}")
            return None
    
    def get_sms_summary(self) -> Dict[str, any]:
        """Get summary information about SMS messages."""
        try:
            sms_messages = self.content_provider.get_sms_messages(limit=1000)  # Sample for stats
            
            if not sms_messages:
                return {"total": 0, "has_access": False}
            
            # Calculate statistics
            message_types = {}
            read_count = 0
            unread_count = 0
            thread_count = set()
            
            for message in sms_messages:
                msg_type = message.get("message_type", "unknown")
                message_types[msg_type] = message_types.get(msg_type, 0) + 1
                
                if message.get("is_read", False):
                    read_count += 1
                else:
                    unread_count += 1
                
                thread_id = message.get("thread_id", "")
                if thread_id:
                    thread_count.add(thread_id)
            
            return {
                "total": len(sms_messages),
                "message_types": message_types,
                "read_count": read_count,
                "unread_count": unread_count,
                "conversation_count": len(thread_count),
                "has_access": True
            }
            
        except Exception as e:
            logger.error(f"Failed to get SMS summary: {e}")
            return {"total": 0, "has_access": False, "error": str(e)}
    
    def export_by_conversation(
        self,
        output_dir: Path,
        format: str = "json"
    ) -> Dict[str, str]:
        """Export SMS messages grouped by conversation."""
        logger.info("Starting SMS export by conversation...")
        
        sms_messages = self.content_provider.get_sms_messages()
        
        if not sms_messages:
            logger.warning("No SMS messages found or access denied")
            return {}
        
        # Group messages by thread_id
        conversations = {}
        for message in sms_messages:
            thread_id = message.get("thread_id", "unknown")
            if thread_id not in conversations:
                conversations[thread_id] = []
            conversations[thread_id].append(message)
        
        ensure_directory(output_dir)
        exported_files = {}
        
        if format == "json":
            json_file = output_dir / "sms_conversations.json"
            
            export_data = {
                "export_timestamp": datetime.now().isoformat(),
                "total_conversations": len(conversations),
                "total_messages": len(sms_messages),
                "device_serial": self.device.serial,
                "conversations": conversations
            }
            
            try:
                with open(json_file, "w", encoding="utf-8") as f:
                    json.dump(export_data, f, indent=2, ensure_ascii=False)
                
                exported_files["json"] = str(json_file)
                logger.info(f"Exported {len(conversations)} conversations to {json_file}")
                
            except Exception as e:
                logger.error(f"Failed to export conversations: {e}")
        
        return exported_files
    
    def test_access(self) -> bool:
        """Test if SMS can be accessed."""
        try:
            access_results = self.content_provider.test_content_provider_access()
            return access_results.get("sms", False)
        except Exception:
            return False