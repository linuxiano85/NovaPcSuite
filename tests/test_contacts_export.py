"""Tests for contacts export functionality."""

from unittest.mock import MagicMock, patch

import pytest

from nova.data.contacts import ContactsExporter


class TestContactsExporter:
    """Test contacts export functionality."""
    
    def test_exporter_creation(self):
        """Test creating a contacts exporter."""
        mock_device = MagicMock()
        exporter = ContactsExporter(mock_device)
        
        assert exporter.device == mock_device
    
    def test_create_vcard(self):
        """Test vCard creation."""
        mock_device = MagicMock()
        exporter = ContactsExporter(mock_device)
        
        contact = {
            "_id": "123",
            "display_name": "John Doe",
            "phone_numbers": ["+1234567890", "+0987654321"],
            "email_addresses": ["john@example.com", "johndoe@test.com"],
            "times_contacted": "5"
        }
        
        vcard = exporter._create_vcard(contact)
        
        assert "BEGIN:VCARD" in vcard
        assert "END:VCARD" in vcard
        assert "FN:John Doe" in vcard
        assert "N:Doe;John;;;" in vcard
        assert "TEL:+1234567890" in vcard
        assert "EMAIL:john@example.com" in vcard
        assert "UID:123" in vcard
    
    def test_clean_phone_number(self):
        """Test phone number cleaning."""
        mock_device = MagicMock()
        exporter = ContactsExporter(mock_device)
        
        # Test various phone number formats
        assert exporter._clean_phone_number("+1 (234) 567-8900") == "+1 (234) 567-8900"
        assert exporter._clean_phone_number("1234567890") == "1234567890"
        assert exporter._clean_phone_number("+1-234-567-8900") == "+1-234-567-8900"
    
    @patch('nova.data.contacts.ContentProvider')
    def test_get_contacts_summary(self, mock_content_provider):
        """Test getting contacts summary."""
        mock_device = MagicMock()
        
        # Mock contacts data
        mock_contacts = [
            {
                "_id": "1",
                "display_name": "John Doe",
                "phone_numbers": ["+1234567890"],
                "email_addresses": ["john@example.com"]
            },
            {
                "_id": "2", 
                "display_name": "Jane Smith",
                "phone_numbers": [],
                "email_addresses": ["jane@example.com"]
            },
            {
                "_id": "3",
                "display_name": "Bob Wilson",
                "phone_numbers": ["+0987654321"],
                "email_addresses": []
            }
        ]
        
        mock_content_provider_instance = MagicMock()
        mock_content_provider_instance.get_contacts.return_value = mock_contacts
        mock_content_provider.return_value = mock_content_provider_instance
        
        exporter = ContactsExporter(mock_device)
        summary = exporter.get_contacts_summary()
        
        assert summary["total"] == 3
        assert summary["with_phone"] == 2  # John and Bob have phone numbers
        assert summary["with_email"] == 2  # John and Jane have email addresses
        assert summary["has_access"] == True