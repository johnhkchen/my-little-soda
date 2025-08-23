# Claude Development Notes

## Documentation Maintenance

⚠️ **README Maintenance Reminder**: The README is a living document that must be kept current.

### When to Update README:
- **Command changes**: Update examples when CLI interface changes
- **Feature additions**: Document new functionality and workflows  
- **Version releases**: Update version badges and feature availability
- **Link changes**: Verify external links during releases
- **Setup process changes**: Update installation/setup instructions

### Pre-Release Checklist:
- [ ] Verify all command examples work with current build
- [ ] Check that version numbers match Cargo.toml
- [ ] Test all external links are accessible
- [ ] Ensure feature descriptions match actual functionality
- [ ] Validate setup instructions with fresh repository

### Quick README Verification:
```bash
# Test that key commands work as documented
./target/release/my-little-soda --help
./target/release/my-little-soda pop --help
./target/release/my-little-soda bottle --help
./target/release/my-little-soda init --help

# Verify repository functionality
./target/release/my-little-soda status
```

**Remember**: Documentation debt is technical debt. Fix it promptly when discovered.