"""Backup rules engine (stub for future implementation)."""

from typing import Dict, List, Optional

from ..util.logging import get_logger

logger = get_logger(__name__)


class BackupRule:
    """Base class for backup rules."""
    
    def __init__(self, name: str, description: str = ""):
        self.name = name
        self.description = description
        self.enabled = True
    
    def should_include(self, file_info: Dict[str, any]) -> bool:
        """Determine if a file should be included in backup."""
        raise NotImplementedError
    
    def get_priority(self, file_info: Dict[str, any]) -> int:
        """Get backup priority for a file (higher = more important)."""
        return 0


class SizeBasedRule(BackupRule):
    """Rule based on file size."""
    
    def __init__(self, max_size_mb: int):
        super().__init__(f"Size limit ({max_size_mb}MB)", f"Exclude files larger than {max_size_mb}MB")
        self.max_size_bytes = max_size_mb * 1024 * 1024
    
    def should_include(self, file_info: Dict[str, any]) -> bool:
        file_size = file_info.get("size", 0)
        return file_size <= self.max_size_bytes


class ExtensionBasedRule(BackupRule):
    """Rule based on file extension."""
    
    def __init__(self, extensions: List[str], include: bool = True):
        action = "Include" if include else "Exclude"
        super().__init__(f"{action} extensions", f"{action} files with extensions: {', '.join(extensions)}")
        self.extensions = [ext.lower() for ext in extensions]
        self.include = include
    
    def should_include(self, file_info: Dict[str, any]) -> bool:
        file_path = file_info.get("path", "").lower()
        has_extension = any(file_path.endswith(ext) for ext in self.extensions)
        
        if self.include:
            return has_extension
        else:
            return not has_extension


class PathBasedRule(BackupRule):
    """Rule based on file path patterns."""
    
    def __init__(self, patterns: List[str], include: bool = True):
        action = "Include" if include else "Exclude"
        super().__init__(f"{action} paths", f"{action} files matching patterns: {', '.join(patterns)}")
        self.patterns = patterns
        self.include = include
    
    def should_include(self, file_info: Dict[str, any]) -> bool:
        import fnmatch
        
        file_path = file_info.get("path", "")
        matches_pattern = any(fnmatch.fnmatch(file_path, pattern) for pattern in self.patterns)
        
        if self.include:
            return matches_pattern
        else:
            return not matches_pattern


class CategoryPriorityRule(BackupRule):
    """Rule that assigns priority based on file category."""
    
    def __init__(self, category_priorities: Dict[str, int]):
        super().__init__("Category priority", "Assign backup priority based on file category")
        self.category_priorities = category_priorities
    
    def should_include(self, file_info: Dict[str, any]) -> bool:
        # This rule doesn't exclude files, just assigns priority
        return True
    
    def get_priority(self, file_info: Dict[str, any]) -> int:
        from .manifest import categorize_file
        
        file_path = file_info.get("path", "")
        category = categorize_file(file_path)
        
        return self.category_priorities.get(category, 0)


class RuleEngine:
    """Engine for applying backup rules."""
    
    def __init__(self):
        self.rules: List[BackupRule] = []
        self._setup_default_rules()
    
    def _setup_default_rules(self):
        """Setup default rules."""
        # Default size limit
        self.add_rule(SizeBasedRule(1024))  # 1GB limit
        
        # Exclude common temporary/cache files
        self.add_rule(ExtensionBasedRule([".tmp", ".cache", ".log"], include=False))
        
        # Exclude system paths
        self.add_rule(PathBasedRule([
            "/system/*",
            "/proc/*", 
            "/dev/*",
            "*/cache/*",
            "*/.thumbnails/*"
        ], include=False))
        
        # Priority for different file types
        self.add_rule(CategoryPriorityRule({
            "image": 100,
            "video": 90,
            "document": 80,
            "audio": 70,
            "messaging": 85,
            "other": 50
        }))
    
    def add_rule(self, rule: BackupRule):
        """Add a backup rule."""
        self.rules.append(rule)
        logger.debug(f"Added backup rule: {rule.name}")
    
    def remove_rule(self, rule_name: str):
        """Remove a backup rule by name."""
        self.rules = [r for r in self.rules if r.name != rule_name]
        logger.debug(f"Removed backup rule: {rule_name}")
    
    def should_include_file(self, file_info: Dict[str, any]) -> bool:
        """Determine if a file should be included based on all rules."""
        for rule in self.rules:
            if not rule.enabled:
                continue
                
            if not rule.should_include(file_info):
                logger.debug(f"File excluded by rule '{rule.name}': {file_info.get('path', '')}")
                return False
        
        return True
    
    def get_file_priority(self, file_info: Dict[str, any]) -> int:
        """Get backup priority for a file based on all rules."""
        max_priority = 0
        
        for rule in self.rules:
            if rule.enabled:
                priority = rule.get_priority(file_info)
                max_priority = max(max_priority, priority)
        
        return max_priority
    
    def filter_files(self, files: List[Dict[str, any]]) -> List[Dict[str, any]]:
        """Filter files based on all rules."""
        filtered_files = []
        
        for file_info in files:
            if self.should_include_file(file_info):
                # Add priority to file info
                file_info["backup_priority"] = self.get_file_priority(file_info)
                filtered_files.append(file_info)
        
        # Sort by priority (highest first)
        filtered_files.sort(key=lambda x: x.get("backup_priority", 0), reverse=True)
        
        return filtered_files
    
    def get_rule_summary(self) -> List[Dict[str, any]]:
        """Get summary of all rules."""
        return [
            {
                "name": rule.name,
                "description": rule.description,
                "enabled": rule.enabled,
                "type": rule.__class__.__name__
            }
            for rule in self.rules
        ]