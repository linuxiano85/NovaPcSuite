"""Command Line Interface for NovaPcSuite."""

import sys
from pathlib import Path
from typing import List, Optional

import click
from rich.console import Console
from rich.table import Table

from .adb import ADBError, check_adb_available, list_devices
from .apps import APKBackup
from .backup import BackupExecutor, BackupStorage, DeviceScanner, RestoreExecutor
from .config import get_config, load_config, save_config
from .data import CallLogExporter, ContactsExporter, SMSExporter
from .util import get_logger, setup_logging

console = Console()


def setup_cli_logging(verbose: bool = False):
    """Setup logging for CLI."""
    level = "DEBUG" if verbose else "INFO"
    setup_logging(level=level, console=console)


@click.group()
@click.option("--verbose", "-v", is_flag=True, help="Enable verbose logging")
@click.option("--config", "-c", type=click.Path(exists=True, path_type=Path), help="Configuration file path")
@click.pass_context
def cli(ctx, verbose: bool, config: Optional[Path]):
    """NovaPcSuite - Linux-centric Android device management and backup suite."""
    setup_cli_logging(verbose)
    
    # Load configuration
    if config:
        ctx.ensure_object(dict)
        ctx.obj["config"] = load_config(config)
    
    # Check ADB availability
    if not check_adb_available():
        console.print("[red]Error: ADB is not available or not in PATH[/red]")
        console.print("Please ensure Android Debug Bridge (ADB) is installed and accessible.")
        sys.exit(1)


@cli.group()
def device():
    """Device management commands."""
    pass


@device.command("info")
@click.option("--serial", "-s", help="Device serial number")
def device_info(serial: Optional[str]):
    """Show device information."""
    try:
        devices = list_devices()
        
        if not devices:
            console.print("[yellow]No devices found[/yellow]")
            return
        
        target_device = None
        if serial:
            target_device = next((d for d in devices if d.serial == serial), None)
            if not target_device:
                console.print(f"[red]Device with serial {serial} not found[/red]")
                return
        elif len(devices) == 1:
            target_device = devices[0]
        else:
            console.print("[yellow]Multiple devices found. Please specify --serial[/yellow]")
            _list_devices(devices)
            return
        
        # Get detailed device info
        device_info = target_device.get_device_info()
        storage_info = target_device.get_storage_info()
        
        table = Table(title=f"Device Information - {device_info.display_name}")
        table.add_column("Property", style="cyan")
        table.add_column("Value", style="white")
        
        table.add_row("Serial", device_info.serial)
        table.add_row("Brand", device_info.brand)
        table.add_row("Model", device_info.model)
        table.add_row("Android Version", device_info.android_version)
        table.add_row("SDK Version", device_info.sdk_version)
        table.add_row("State", device_info.state)
        table.add_row("Root Access", "Yes" if target_device.has_root() else "No")
        
        # Storage information
        if storage_info.get("total", 0) > 0:
            from .util.paths import format_size
            table.add_row("Storage Total", format_size(storage_info["total"]))
            table.add_row("Storage Used", format_size(storage_info["used"]))
            table.add_row("Storage Available", format_size(storage_info["available"]))
        
        console.print(table)
        
    except ADBError as e:
        console.print(f"[red]ADB Error: {e}[/red]")
    except Exception as e:
        console.print(f"[red]Error: {e}[/red]")


@device.command("list")
def device_list():
    """List connected devices."""
    try:
        devices = list_devices()
        _list_devices(devices)
    except ADBError as e:
        console.print(f"[red]ADB Error: {e}[/red]")


def _list_devices(devices):
    """Helper to display device list."""
    if not devices:
        console.print("[yellow]No devices found[/yellow]")
        return
    
    table = Table(title="Connected Devices")
    table.add_column("Serial", style="cyan")
    table.add_column("Model", style="white")
    table.add_column("Brand", style="white")
    table.add_column("Android", style="white")
    table.add_column("State", style="green")
    
    for device in devices:
        try:
            info = device.get_device_info()
            table.add_row(info.serial, info.model, info.brand, info.android_version, info.state)
        except Exception:
            table.add_row(device.serial, "Unknown", "Unknown", "Unknown", "Unknown")
    
    console.print(table)


@device.command("oem-info")
@click.option("--serial", "-s", help="Device serial number")
def device_oem_info(serial: Optional[str]):
    """Show OEM/bootloader information and unlock guidance."""
    console.print("[bold cyan]OEM/Bootloader Information[/bold cyan]\n")
    
    console.print("[yellow]Note: This is a stub implementation.[/yellow]")
    console.print("Full bootloader unlock automation requires OEM-specific portals and cannot be fully implemented locally.\n")
    
    console.print("[bold]General Bootloader Unlock Process:[/bold]")
    console.print("1. Enable Developer Options and OEM Unlocking")
    console.print("2. Boot device into fastboot mode")
    console.print("3. Check unlock ability: [cyan]fastboot flashing get_unlock_ability[/cyan]")
    console.print("4. Visit OEM unlock portal (varies by manufacturer)")
    console.print("5. Follow OEM-specific unlock procedure\n")
    
    console.print("[bold red]Warning:[/bold red] Unlocking bootloader will:")
    console.print("- Void warranty")
    console.print("- Erase all user data")
    console.print("- Potentially brick device if done incorrectly")
    console.print("- Make device less secure\n")
    
    console.print("[bold]Common OEM Portals:[/bold]")
    console.print("- Xiaomi: https://en.miui.com/unlock/")
    console.print("- OnePlus: https://www.oneplus.com/support/answer/detail/op289")
    console.print("- Samsung: Not officially supported")
    console.print("- Google: Built-in fastboot unlock")


@cli.group()
def backup():
    """Backup management commands."""
    pass


@backup.command("run")
@click.option("--serial", "-s", help="Device serial number")
@click.option("--include", multiple=True, help="Include specific paths")
@click.option("--exclude", multiple=True, help="Exclude file patterns")
@click.option("--no-apk", is_flag=True, help="Skip APK backup")
@click.option("--no-contacts", is_flag=True, help="Skip contacts export")
@click.option("--no-logs", is_flag=True, help="Skip call logs and SMS export")
@click.option("--backup-id", help="Custom backup ID")
def backup_run(serial: Optional[str], include: List[str], exclude: List[str], 
               no_apk: bool, no_contacts: bool, no_logs: bool, backup_id: Optional[str]):
    """Run a complete backup operation."""
    try:
        # Get target device
        device = _get_target_device(serial)
        if not device:
            return
        
        logger = get_logger(__name__)
        logger.info(f"Starting backup for device: {device.serial}")
        
        # Initialize components
        scanner = DeviceScanner(device)
        executor = BackupExecutor(device)
        
        # Configure scan paths
        config = get_config()
        scan_paths = list(include) if include else config.scanner.include_paths
        exclude_patterns = list(exclude) if exclude else config.scanner.exclude_patterns
        
        console.print(f"[bold cyan]Starting backup for {device.get_device_info().display_name}[/bold cyan]")
        
        # Scan device
        console.print("[yellow]Scanning device files...[/yellow]")
        scan_result = scanner.scan_device(
            include_paths=scan_paths,
            exclude_patterns=exclude_patterns
        )
        
        if scan_result.errors:
            console.print(f"[yellow]Scan warnings: {len(scan_result.errors)} errors[/yellow]")
        
        from .util.paths import format_size
        console.print(f"Found {scan_result.total_files} files ({format_size(scan_result.total_size)})")
        
        # Execute backup
        console.print("[yellow]Backing up files...[/yellow]")
        manifest = executor.execute_backup(scan_result, backup_id=backup_id)
        
        if not manifest:
            console.print("[red]Backup failed[/red]")
            return
        
        # APK backup
        if not no_apk:
            console.print("[yellow]Backing up APKs...[/yellow]")
            from .adb.package import PackageManager
            package_manager = PackageManager(device)
            packages = package_manager.list_packages(include_system=False)
            
            if packages:
                from .util.paths import get_backup_path
                backup_path = get_backup_path(device.serial, manifest.created_at.split('T')[0].replace('-', '') + '_' + manifest.created_at.split('T')[1][:6].replace(':', ''))
                apk_count = executor.backup_apks(packages, backup_path, manifest, executor.manifest_manager)
                console.print(f"Backed up {apk_count} APKs")
        
        # Data exports
        backup_path = get_backup_path(device.serial, backup_id or manifest.id)
        
        if not no_contacts:
            console.print("[yellow]Exporting contacts...[/yellow]")
            contacts_exporter = ContactsExporter(device)
            contacts_files = contacts_exporter.export_contacts(backup_path / "contacts")
            for format_type, file_path in contacts_files.items():
                console.print(f"Contacts exported: {format_type} -> {file_path}")
        
        if not no_logs:
            console.print("[yellow]Exporting call logs and SMS...[/yellow]")
            
            # Call logs
            calllog_exporter = CallLogExporter(device)
            calllog_files = calllog_exporter.export_call_log(backup_path / "logs")
            for format_type, file_path in calllog_files.items():
                console.print(f"Call logs exported: {format_type} -> {file_path}")
            
            # SMS
            sms_exporter = SMSExporter(device)
            sms_files = sms_exporter.export_sms(backup_path / "logs")
            for format_type, file_path in sms_files.items():
                console.print(f"SMS exported: {format_type} -> {file_path}")
        
        console.print(f"[bold green]Backup completed successfully![/bold green]")
        console.print(f"Backup ID: {manifest.id}")
        console.print(f"Location: {backup_path}")
        
    except Exception as e:
        console.print(f"[red]Backup failed: {e}[/red]")


@backup.command("list")
@click.option("--serial", "-s", help="Filter by device serial")
def backup_list(serial: Optional[str]):
    """List available backups."""
    try:
        storage = BackupStorage()
        
        if serial:
            backups = storage.list_device_backups(serial)
        else:
            backups = storage.list_all_backups()
        
        if not backups:
            console.print("[yellow]No backups found[/yellow]")
            return
        
        table = Table(title="Available Backups")
        table.add_column("ID", style="cyan")
        table.add_column("Device", style="white")
        table.add_column("Date", style="white")
        table.add_column("Files", style="white")
        table.add_column("Size", style="white")
        table.add_column("APKs", style="white")
        
        for backup in backups:
            from .util.paths import format_size
            table.add_row(
                backup["id"][:8] + "...",
                backup["device_name"],
                backup["timestamp"],
                str(backup["total_files"]),
                format_size(backup.get("actual_size", backup["total_size"])),
                str(backup["total_apks"])
            )
        
        console.print(table)
        
    except Exception as e:
        console.print(f"[red]Error listing backups: {e}[/red]")


@backup.command("show")
@click.argument("backup_id")
def backup_show(backup_id: str):
    """Show detailed information about a backup."""
    try:
        storage = BackupStorage()
        backup = storage.get_backup_by_id(backup_id)
        
        if not backup:
            console.print(f"[red]Backup not found: {backup_id}[/red]")
            return
        
        table = Table(title=f"Backup Details - {backup_id}")
        table.add_column("Property", style="cyan")
        table.add_column("Value", style="white")
        
        from .util.paths import format_size
        
        table.add_row("ID", backup["id"])
        table.add_row("Device", backup["device_name"])
        table.add_row("Serial", backup["device_serial"])
        table.add_row("Created", backup["created_at"])
        table.add_row("Total Files", str(backup["total_files"]))
        table.add_row("Total Size", format_size(backup["total_size"]))
        table.add_row("Actual Size", format_size(backup.get("actual_size", 0)))
        table.add_row("APKs", str(backup["total_apks"]))
        table.add_row("Incremental", "Yes" if backup.get("incremental") else "No")
        table.add_row("Location", backup["path"])
        
        console.print(table)
        
    except Exception as e:
        console.print(f"[red]Error showing backup: {e}[/red]")


def _get_target_device(serial: Optional[str]):
    """Get target device for operations."""
    try:
        devices = list_devices()
        
        if not devices:
            console.print("[red]No devices found[/red]")
            return None
        
        if serial:
            device = next((d for d in devices if d.serial == serial), None)
            if not device:
                console.print(f"[red]Device with serial {serial} not found[/red]")
                return None
            return device
        elif len(devices) == 1:
            return devices[0]
        else:
            console.print("[yellow]Multiple devices found. Please specify --serial[/yellow]")
            _list_devices(devices)
            return None
            
    except ADBError as e:
        console.print(f"[red]ADB Error: {e}[/red]")
        return None


@cli.group()
def restore():
    """Restore management commands."""
    pass


@restore.command("files")
@click.argument("backup_id")
@click.option("--target-dir", "-t", type=click.Path(path_type=Path), help="Target directory for restore")
@click.option("--pattern", "-p", multiple=True, help="File patterns to restore")
def restore_files(backup_id: str, target_dir: Optional[Path], pattern: List[str]):
    """Restore files from backup."""
    if not target_dir:
        console.print("[red]Target directory is required for file restore[/red]")
        return
    
    try:
        # For now, we only support local restore since ADB push is complex
        from .backup import RestoreExecutor
        from .adb.device import ADBDevice
        
        # Create a dummy device for RestoreExecutor (we're not actually using it)
        devices = list_devices()
        if not devices:
            console.print("[yellow]No devices connected (required for restore executor)[/yellow]")
            return
        
        executor = RestoreExecutor(devices[0])
        
        console.print(f"[yellow]Restoring files from backup {backup_id}...[/yellow]")
        
        result = executor.restore_to_local(
            backup_id=backup_id,
            local_target_dir=target_dir,
            file_patterns=list(pattern) if pattern else None
        )
        
        console.print(f"[bold green]Restore completed![/bold green]")
        console.print(f"Files restored: {result['success_count']}/{result['total_files']}")
        
        if result['failed_count'] > 0:
            console.print(f"[yellow]Failed files: {result['failed_count']}[/yellow]")
        
    except Exception as e:
        console.print(f"[red]Restore failed: {e}[/red]")


@cli.group()
def export():
    """Data export commands."""
    pass


@export.command("contacts")
@click.option("--serial", "-s", help="Device serial number")
@click.option("--format", "-f", multiple=True, default=["vcf", "csv"], help="Export formats")
@click.option("--output", "-o", type=click.Path(path_type=Path), help="Output directory")
def export_contacts(serial: Optional[str], format: List[str], output: Optional[Path]):
    """Export contacts from device."""
    try:
        device = _get_target_device(serial)
        if not device:
            return
        
        if not output:
            output = Path.cwd() / "exports" / "contacts"
        
        console.print("[yellow]Exporting contacts...[/yellow]")
        
        exporter = ContactsExporter(device)
        files = exporter.export_contacts(output, list(format))
        
        if files:
            console.print("[bold green]Contacts exported successfully![/bold green]")
            for fmt, file_path in files.items():
                console.print(f"{fmt.upper()}: {file_path}")
        else:
            console.print("[yellow]No contacts exported (access denied or no contacts found)[/yellow]")
        
    except Exception as e:
        console.print(f"[red]Export failed: {e}[/red]")


@export.command("logs")
@click.option("--serial", "-s", help="Device serial number")
@click.option("--calls", is_flag=True, help="Export call logs only")
@click.option("--sms", is_flag=True, help="Export SMS only")
@click.option("--output", "-o", type=click.Path(path_type=Path), help="Output directory")
def export_logs(serial: Optional[str], calls: bool, sms: bool, output: Optional[Path]):
    """Export call logs and SMS from device."""
    try:
        device = _get_target_device(serial)
        if not device:
            return
        
        if not output:
            output = Path.cwd() / "exports" / "logs"
        
        # Export both if neither specified
        if not calls and not sms:
            calls = sms = True
        
        if calls:
            console.print("[yellow]Exporting call logs...[/yellow]")
            exporter = CallLogExporter(device)
            files = exporter.export_call_log(output)
            
            if files:
                console.print("[bold green]Call logs exported![/bold green]")
                for fmt, file_path in files.items():
                    console.print(f"Call logs {fmt.upper()}: {file_path}")
        
        if sms:
            console.print("[yellow]Exporting SMS...[/yellow]")
            exporter = SMSExporter(device)
            files = exporter.export_sms(output)
            
            if files:
                console.print("[bold green]SMS exported![/bold green]")
                for fmt, file_path in files.items():
                    console.print(f"SMS {fmt.upper()}: {file_path}")
        
    except Exception as e:
        console.print(f"[red]Export failed: {e}[/red]")


@cli.group()
def apps():
    """Application management commands."""
    pass


@apps.command("list")
@click.option("--serial", "-s", help="Device serial number")
@click.option("--system", is_flag=True, help="Include system apps")
def apps_list(serial: Optional[str], system: bool):
    """List installed applications."""
    try:
        device = _get_target_device(serial)
        if not device:
            return
        
        apk_backup = APKBackup(device)
        packages = apk_backup.list_installed_packages(include_system=system)
        
        table = Table(title=f"Installed Applications ({'All' if system else 'User Only'})")
        table.add_column("Package", style="cyan")
        table.add_column("Version", style="white")
        table.add_column("Type", style="white")
        table.add_column("Enabled", style="white")
        
        for pkg in packages:
            pkg_type = "System" if pkg["is_system"] else "User"
            enabled = "Yes" if pkg["is_enabled"] else "No"
            
            table.add_row(
                pkg["package"],
                pkg["version_name"],
                pkg_type,
                enabled
            )
        
        console.print(table)
        
    except Exception as e:
        console.print(f"[red]Error listing apps: {e}[/red]")


@apps.command("backup")
@click.option("--serial", "-s", help="Device serial number")
@click.option("--packages", "-p", multiple=True, help="Specific packages to backup")
@click.option("--output", "-o", type=click.Path(path_type=Path), help="Output directory")
def apps_backup(serial: Optional[str], packages: List[str], output: Optional[Path]):
    """Backup application APK files."""
    try:
        device = _get_target_device(serial)
        if not device:
            return
        
        if not output:
            output = Path.cwd() / "apk_backup"
        
        apk_backup = APKBackup(device)
        
        if packages:
            target_packages = list(packages)
        else:
            # Backup all user apps
            all_packages = apk_backup.list_installed_packages(include_system=False)
            target_packages = [pkg["package"] for pkg in all_packages]
        
        console.print(f"[yellow]Backing up {len(target_packages)} packages...[/yellow]")
        
        result = apk_backup.backup_packages(target_packages, output)
        
        console.print(f"[bold green]APK backup completed![/bold green]")
        console.print(f"Successful: {result['success']}/{result['total']}")
        
        if result['failed'] > 0:
            console.print(f"[yellow]Failed: {result['failed']}[/yellow]")
            for error in result['errors'][:5]:  # Show first 5 errors
                console.print(f"  {error}")
        
    except Exception as e:
        console.print(f"[red]APK backup failed: {e}[/red]")


def main():
    """Main entry point."""
    cli()


if __name__ == "__main__":
    main()