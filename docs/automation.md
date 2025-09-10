# Repository Automation Guide

This document explains the automated workflows and governance systems implemented in NovaPcSuite to streamline development and maintain code quality.

## Overview

The repository includes comprehensive automation for:
- **Label Management**: Centralized label taxonomy with automatic synchronization
- **Auto-Merge**: Conditional automatic merging based on labels and status
- **Code Ownership**: Structured review requirements for critical components
- **Quality Gates**: Automated checks and validations

## Label System

### Label Categories

Our label system uses a comprehensive taxonomy organized into categories:

#### Type Labels (`type:*`)
- `type:bug` - Bug fixes
- `type:feature` - New functionality
- `type:enhancement` - Improvements to existing features
- `type:docs` - Documentation changes
- `type:refactor` - Code refactoring
- `type:test` - Test additions or improvements
- `type:ci` - CI/CD pipeline changes
- `type:chore` - Maintenance tasks

#### Area Labels (`area:*`)
- `area:core` - Core application logic
- `area:plugin-system` - Plugin architecture and API
- `area:ui` - User interface components
- `area:backup` - Backup functionality
- `area:restore` - Restore functionality
- `area:recovery` - Data recovery features
- `area:encryption` - Encryption and security
- `area:telephony` - Telephony integration
- `area:scheduler` - Task scheduling
- `area:config` - Configuration management
- `area:transport` - Data transport and sync
- `area:docs` - Documentation

#### Status Labels (`status:*`)
- `status:wip` - Work in progress (blocks auto-merge)
- `status:blocked` - Blocked by dependencies (blocks auto-merge)
- `status:needs-review` - Needs code review
- `status:needs-testing` - Needs testing
- `status:merge-ready` - Ready for merge (enables auto-merge)
- `status:on-hold` - On hold
- `status:duplicate` - Duplicate issue/PR
- `status:invalid` - Invalid or not reproducible
- `status:wontfix` - Will not be fixed

#### Priority Labels (`priority:*`)
- `priority:critical` - Critical priority
- `priority:high` - High priority
- `priority:medium` - Medium priority
- `priority:low` - Low priority

#### Risk Labels (`risk:*`)
- `risk:breaking` - Breaking change
- `risk:high` - High risk change
- `risk:medium` - Medium risk change
- `risk:low` - Low risk change

#### Security Labels (`security:*`)
- `security:vulnerability` - Security vulnerability
- `security:audit` - Security audit required
- `security:sensitive` - Contains sensitive changes
- `security:compliance` - Compliance related

#### Automation Labels (`automation:*`)
- `automation:auto-merge` - Eligible for auto-merge
- `automation:skip-ci` - Skip CI pipeline
- `automation:release` - Related to release automation
- `automation:dependencies` - Dependency updates

#### Meta Labels (`meta:*`)
- `meta:good-first-issue` - Good for newcomers
- `meta:help-wanted` - Extra attention needed
- `meta:question` - Question or discussion
- `meta:epic` - Epic or large feature
- `meta:tracking` - Tracking issue

#### Size Labels (`size:*`)
- `size:xs` - Extra small changes (<10 lines)
- `size:s` - Small changes (10-50 lines)
- `size:m` - Medium changes (50-200 lines)
- `size:l` - Large changes (200-500 lines)
- `size:xl` - Extra large changes (500+ lines)

### Using Labels

#### Applying Labels
Labels should be applied to issues and PRs according to their content and status:

1. **Always apply a type label** to categorize the change
2. **Apply relevant area labels** to indicate affected components
3. **Use status labels** to track progress and control automation
4. **Add priority labels** for issue triage
5. **Include risk labels** for changes that might affect stability
6. **Add security labels** for security-related changes

#### Label Sync Workflow
Labels are automatically synchronized from the central definition file:
- **Trigger**: Changes to `.github/labels.yml` or manual dispatch
- **Action**: Creates/updates labels to match the definition
- **Safety**: Does not delete existing labels not in the config

To manually sync labels:
```bash
gh workflow run "Label Sync" --field dry_run=false
```

## Auto-Merge System

### How It Works

The auto-merge system automatically merges PRs when specific conditions are met, reducing manual intervention for routine changes.

### Conditions for Auto-Merge

A PR is eligible for auto-merge when **ALL** of the following conditions are met:

1. **Required Label**: Must have `status:merge-ready` label
2. **No Blocking Labels**: Must NOT have `status:wip` or `status:blocked` labels
3. **Not Draft**: PR must not be in draft mode
4. **No Conflicts**: PR must be mergeable (no merge conflicts)
5. **CI Success**: All required checks must pass (when CI is implemented)

### Triggering Auto-Merge

To enable auto-merge for a PR:

1. Ensure your PR meets all the conditions above
2. Add the `status:merge-ready` label to your PR
3. The auto-merge workflow will automatically detect and process the PR

### Auto-Merge Strategy

- **Merge Method**: Squash merge (creates a single commit)
- **Branch Cleanup**: Automatically deletes the feature branch
- **Commit Message**: Uses PR title and description

### Overriding Auto-Merge

To prevent auto-merge:
- Add `status:wip` or `status:blocked` label
- Convert PR to draft
- Remove `status:merge-ready` label

### Manual Override

Repository maintainers can always merge manually regardless of auto-merge status.

## Code Ownership (CODEOWNERS)

### Ownership Structure

All critical directories require review from `@linuxiano85`:

- **Core Components**: `nova-core/`, `nova-plugin-api/`, `nova-ui/`
- **Plugin System**: `plugins/`
- **Security Areas**: `encryption/`, `crypto/` (future)
- **Critical Functions**: `backup/`, `restore/`, `recovery/` (future)
- **Integration Points**: `telephony/`, `transport/` (future)
- **Configuration**: `docs/`, `*.md`, `*.yml`, `*.toml`

### Review Requirements

- All PRs touching owned files require approval from the code owner
- Reviews ensure architectural consistency and security compliance
- Owners are automatically requested as reviewers

## Mergify Integration (Optional)

The repository includes a `.mergify.yml` configuration that replicates the auto-merge logic for users who prefer Mergify. This configuration is inert unless Mergify is explicitly enabled.

### Additional Mergify Features

- **Auto-labeling**: Automatically applies labels based on file changes
- **Size detection**: Adds size labels based on lines changed
- **Security flagging**: Flags potentially sensitive changes
- **Breaking change detection**: Identifies potential breaking changes

## Workflow Examples

### Typical Feature Development

1. Create feature branch
2. Implement changes
3. Create PR with appropriate labels:
   - `type:feature`
   - `area:*` (relevant area)
   - `priority:*` (if applicable)
4. Request review or wait for automatic review assignment
5. Address feedback and add `status:merge-ready` when complete
6. Auto-merge activates once all conditions are met

### Bug Fix Process

1. Create bug fix branch
2. Implement fix
3. Create PR with labels:
   - `type:bug`
   - `priority:*` (based on severity)
   - `area:*` (affected component)
4. Add `status:merge-ready` after testing
5. Auto-merge processes the PR

### Security Changes

1. Create security branch
2. Implement security fix
3. Create PR with labels:
   - `type:bug` or `type:enhancement`
   - `security:*` (appropriate security label)
   - `area:*` (affected area)
4. Ensure thorough review due to CODEOWNERS
5. Add `status:merge-ready` after security review
6. Auto-merge processes after all checks

## Future Enhancements

### Planned Automation Features

1. **CI/CD Pipeline**
   - Automated building and testing
   - Security scanning with `cargo audit`
   - Code quality checks

2. **Release Automation**
   - Automatic version bumping
   - Release note generation
   - Crate publishing to crates.io

3. **Security Auditing**
   - Dependency vulnerability scanning
   - Security compliance checks
   - Automated security reporting

4. **Advanced Auto-Merge**
   - Integration with CI pipeline status
   - Dependency update automation
   - Performance regression detection

### Configuration Expansion

As the project grows, the automation system will be extended to support:

- **Feature flags** for controlling automation behavior
- **Environment-specific** workflows (staging, production)
- **Integration testing** requirements
- **Compliance checking** for regulatory requirements

## Troubleshooting

### Auto-Merge Not Working

Check the following:

1. **Labels**: Ensure `status:merge-ready` is present and no blocking labels exist
2. **Draft Status**: Confirm PR is not in draft mode
3. **Conflicts**: Resolve any merge conflicts
4. **Permissions**: Verify repository settings allow auto-merge
5. **Branch Protection**: Check if branch protection rules are met

### Label Sync Issues

- **Permission Errors**: Ensure GitHub token has sufficient permissions
- **Syntax Errors**: Validate `.github/labels.yml` syntax
- **Rate Limits**: GitHub API may temporarily limit requests

### CODEOWNERS Not Working

- **File Location**: Ensure `.github/CODEOWNERS` is in the correct location
- **Syntax**: Verify file syntax follows GitHub CODEOWNERS format
- **User Existence**: Confirm referenced users exist and have repository access

## Getting Help

- **Issues**: Create an issue with the `meta:question` label
- **Discussions**: Use GitHub Discussions for broader topics
- **Direct Contact**: Reach out to `@linuxiano85` for urgent matters

## Contributing to Automation

Improvements to the automation system are welcome! Please:

1. Create an issue to discuss proposed changes
2. Follow the standard PR process
3. Include thorough testing of workflow changes
4. Update this documentation as needed

Remember that automation changes affect the entire development workflow, so they require careful consideration and testing.