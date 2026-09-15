# Contributing to AI Documentation

## For AI Agents Updating Docs

### When to Update AGENT-NOTES.md

**Always add when you discover:**
- An undocumented gotcha that caused errors or confusion
- A constraint that wasn't obvious from reading the code
- A business rule that's enforced outside the code
- A pattern that must be followed but isn't enforced by types
- Security considerations not visible in code structure
- Relationships between entities that aren't obvious

**Example scenarios:**
- "Merchant deletion failed with foreign key error" → Document the cascade behavior
- "Test failed because profiles must have unique names per merchant" → Document the constraint
- "API call failed due to JWT merchant_id mismatch" → Document the auth requirement

### How to Update AGENT-NOTES.md

**1. Add to existing section if it fits:**
```markdown
## Common Gotchas
- [EXISTING] Profile deletion is soft-delete only
- [NEW] Profile names must be unique per merchant (DB constraint)
```

**2. Keep entries concise:**
```markdown
❌ BAD: "When I was working on the merchant update feature, I discovered that 
if you try to change the merchant name without proper validation, the system 
will accept empty strings but then subsequent API calls will fail..."

✅ GOOD: "Merchant names cannot be empty - validated in core layer, not DB"
```

**3. Include concrete examples when helpful:**
```markdown
## Critical Rules
- Business profiles cannot exist without a merchant
  - Foreign key constraint enforces this
  - Always create merchant first, then profiles
```

### What NOT to Document

**Don't document what code already shows:**
- ❌ "Merchant table has a merchant_id column"
- ❌ "The create_merchant function creates merchants"
- ❌ "We use PostgreSQL" (visible in config)

**Don't document standard practices:**
- ❌ "Always handle errors" 
- ❌ "Write tests for new features"
- ❌ "Follow REST conventions"

### Update Format

When updating docs as an AI agent, use this format:

```markdown
## [Section Name]
<!-- AI-UPDATED: 2024-01-20 - Brief reason -->
- New discovery or gotcha
```

This helps humans reviewing PRs understand:
1. What was added by AI
2. When it was discovered
3. Why it was important enough to document

### For Humans Reviewing AI Updates

**Quick Review Checklist:**
- [ ] Is this actually a gotcha or just standard practice?
- [ ] Would a new developer/AI waste time without this info?
- [ ] Is it in the right domain folder?
- [ ] Is it concise and actionable?
- [ ] Does it explain WHY, not WHAT?

**Merge Criteria:**
- Documents non-obvious constraints
- Prevents future errors
- Specific to this codebase
- Not duplicating code comments

### Examples of Good Documentation Updates

**Business Logic Gotcha:**
```markdown
## Common Gotchas  
<!-- AI-UPDATED: 2024-01-20 - Failed profile creation -->
- Profile names must be unique per merchant (enforced by DB unique index)
```

**Architecture Decision:**
```markdown
## Design Decisions
<!-- AI-UPDATED: 2024-01-20 - Discovered during refactoring -->
- Merchant IDs use nanoid format for URL safety and uniqueness
- Never accept client-provided IDs - always generate server-side
```

**Integration Quirk:**
```markdown
## When Working Here
<!-- AI-UPDATED: 2024-01-20 - Auth failures -->
- JWT merchant_id must match the merchant_id in the URL path
- This is validated in JwtAuth::with_merchant() middleware
```

### File Size Management

**When AGENT-NOTES.md exceeds 300 lines:**
1. Create a new topic-specific file
2. Move related gotchas to the new file
3. Add clear header in AGENT-NOTES.md:

```markdown
# Merchants - Agent Notes

> **Note**: Also read RELATIONSHIPS.md and AUTH-GOTCHAS.md in this folder

## Common Gotchas
[remaining gotchas under 300 lines]
```

**Naming new files:**
- Use CAPS for consistency: `RELATIONSHIPS.md`, `AUTH-GOTCHAS.md`
- Keep names topic-specific and clear
- Maximum 3-4 files per domain

### CRITICAL-PATHS.md Guidelines

**IMPORTANT**: The CRITICAL-PATHS.md file is reserved for cross-domain critical information only.

**DO NOT update CRITICAL-PATHS.md unless:**
- The gotcha/pattern affects multiple domains
- It's a system-wide pattern that all domains must follow
- It's a critical security or data integrity rule that spans domains
- It's an architectural decision that impacts the entire system

**Instead, use domain-specific AGENT-NOTES.md for:**
- Domain-specific gotchas and implementation details
- Business logic specific to one area
- Relationships that only affect that domain
- Testing strategies for that domain

**Examples:**
- ❌ **Wrong**: Adding "Reconciliation uses staging entries" to CRITICAL-PATHS.md
- ✅ **Right**: Adding it to reconciliation/AGENT-NOTES.md
- ✅ **Right for CRITICAL-PATHS**: "All monetary values use MinorUnit (i64)" (affects all domains)
- ✅ **Right for CRITICAL-PATHS**: "All tables use soft delete pattern" (system-wide pattern)

### Maintaining Quality

**AI agents should:**
1. Only update when discovering something non-obvious
2. Check if it's already documented elsewhere
3. Place updates in the correct domain folder
4. Keep updates focused on "surprises" not "standards"
5. Cross-reference related domains when relationships exist
6. **ALWAYS prefer domain AGENT-NOTES.md over CRITICAL-PATHS.md**

**Red flags in updates:**
- Too verbose (over 3 lines for a single point)
- Documenting obvious code behavior
- Creating new files instead of updating AGENT-NOTES.md
- Adding opinions instead of facts
- Missing the "why" behind the constraint
- **Adding domain-specific content to CRITICAL-PATHS.md**