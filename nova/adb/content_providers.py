"""ADB content provider utilities for accessing contacts, SMS, etc."""

import csv
import json
from typing import Dict, List, Optional

from .device import ADBDevice, ADBError
from .shell import ShellCommand
from ..util.logging import get_logger

logger = get_logger(__name__)


class ContentProviderError(Exception):
    """Content provider access error."""
    pass


class ContentProvider:
    """Utility for accessing Android content providers via ADB."""
    
    def __init__(self, device: ADBDevice):
        self.device = device
        self.shell = ShellCommand(device)
    
    def query_content_provider(
        self, 
        uri: str, 
        projection: Optional[List[str]] = None,
        selection: Optional[str] = None,
        sort_order: Optional[str] = None
    ) -> List[Dict[str, str]]:
        """Query a content provider and return structured data."""
        try:
            # Build content query command
            cmd = f"content query --uri {uri}"
            
            if projection:
                cmd += f" --projection {','.join(projection)}"
            
            if selection:
                cmd += f" --where \"{selection}\""
            
            if sort_order:
                cmd += f" --sort \"{sort_order}\""
            
            logger.debug(f"Querying content provider: {cmd}")
            output = self.shell.execute(cmd)
            
            return self._parse_content_output(output)
            
        except ADBError as e:
            logger.error(f"Failed to query content provider {uri}: {e}")
            raise ContentProviderError(f"Content provider query failed: {e}") from e
    
    def _parse_content_output(self, output: str) -> List[Dict[str, str]]:
        """Parse content provider query output."""
        results = []
        lines = output.strip().split("\n")
        
        for line in lines:
            if not line.strip() or "Row:" in line:
                continue
            
            # Parse row data (key=value pairs)
            row_data = {}
            
            # Handle different output formats
            if "=" in line:
                # Format: key1=value1, key2=value2, ...
                pairs = []
                current_pair = ""
                in_quotes = False
                
                for char in line:
                    if char == '"' and (not current_pair or current_pair[-1] != "\\"):
                        in_quotes = not in_quotes
                    elif char == "," and not in_quotes:
                        if current_pair.strip():
                            pairs.append(current_pair.strip())
                        current_pair = ""
                        continue
                    
                    current_pair += char
                
                if current_pair.strip():
                    pairs.append(current_pair.strip())
                
                for pair in pairs:
                    if "=" in pair:
                        key, value = pair.split("=", 1)
                        key = key.strip()
                        value = value.strip().strip('"')
                        row_data[key] = value
            
            if row_data:
                results.append(row_data)
        
        return results
    
    def get_contacts(self) -> List[Dict[str, str]]:
        """Get contacts from the device."""
        try:
            # Query contacts with basic information
            contacts = self.query_content_provider(
                "content://com.android.contacts/contacts",
                projection=[
                    "_id",
                    "display_name", 
                    "starred",
                    "times_contacted",
                    "last_time_contacted"
                ]
            )
            
            # Get additional contact data (phone numbers, emails)
            for contact in contacts:
                contact_id = contact.get("_id")
                if contact_id:
                    # Get phone numbers
                    phones = self.query_content_provider(
                        "content://com.android.contacts/data",
                        projection=["data1", "data2"],
                        selection=f"contact_id={contact_id} AND mimetype='vnd.android.cursor.item/phone_v2'"
                    )
                    contact["phone_numbers"] = [p.get("data1", "") for p in phones if p.get("data1")]
                    
                    # Get email addresses
                    emails = self.query_content_provider(
                        "content://com.android.contacts/data",
                        projection=["data1", "data2"],
                        selection=f"contact_id={contact_id} AND mimetype='vnd.android.cursor.item/email_v2'"
                    )
                    contact["email_addresses"] = [e.get("data1", "") for e in emails if e.get("data1")]
            
            logger.info(f"Retrieved {len(contacts)} contacts")
            return contacts
            
        except ContentProviderError as e:
            logger.error(f"Failed to get contacts: {e}")
            return []
    
    def get_call_log(self, limit: Optional[int] = None) -> List[Dict[str, str]]:
        """Get call log from the device."""
        try:
            # Query call log
            call_log = self.query_content_provider(
                "content://call_log/calls",
                projection=[
                    "_id",
                    "number",
                    "date",
                    "duration", 
                    "type",
                    "name",
                    "cached_number_type",
                    "cached_number_label"
                ],
                sort_order="date DESC"
            )
            
            # Convert call types to readable format
            call_types = {
                "1": "incoming",
                "2": "outgoing", 
                "3": "missed",
                "4": "voicemail",
                "5": "rejected",
                "6": "blocked"
            }
            
            for call in call_log:
                call_type = call.get("type", "")
                call["call_type"] = call_types.get(call_type, f"unknown({call_type})")
            
            if limit:
                call_log = call_log[:limit]
            
            logger.info(f"Retrieved {len(call_log)} call log entries")
            return call_log
            
        except ContentProviderError as e:
            logger.error(f"Failed to get call log: {e}")
            return []
    
    def get_sms_messages(self, limit: Optional[int] = None) -> List[Dict[str, str]]:
        """Get SMS messages from the device."""
        try:
            # Query SMS messages
            sms_messages = self.query_content_provider(
                "content://sms",
                projection=[
                    "_id",
                    "address",
                    "body",
                    "date",
                    "date_sent",
                    "type",
                    "read",
                    "thread_id"
                ],
                sort_order="date DESC"
            )
            
            # Convert message types to readable format
            message_types = {
                "1": "inbox",
                "2": "sent",
                "3": "draft",
                "4": "outbox",
                "5": "failed",
                "6": "queued"
            }
            
            for message in sms_messages:
                msg_type = message.get("type", "")
                message["message_type"] = message_types.get(msg_type, f"unknown({msg_type})")
                
                # Convert read status
                read_status = message.get("read", "0")
                message["is_read"] = read_status == "1"
            
            if limit:
                sms_messages = sms_messages[:limit]
            
            logger.info(f"Retrieved {len(sms_messages)} SMS messages")
            return sms_messages
            
        except ContentProviderError as e:
            logger.error(f"Failed to get SMS messages: {e}")
            return []
    
    def test_content_provider_access(self) -> Dict[str, bool]:
        """Test access to various content providers."""
        providers = {
            "contacts": "content://com.android.contacts/contacts",
            "call_log": "content://call_log/calls",
            "sms": "content://sms"
        }
        
        results = {}
        
        for name, uri in providers.items():
            try:
                # Try to query with limit to test access
                self.query_content_provider(uri + "?limit=1")
                results[name] = True
                logger.debug(f"Content provider access OK: {name}")
            except ContentProviderError:
                results[name] = False
                logger.warning(f"Content provider access denied: {name}")
        
        return results