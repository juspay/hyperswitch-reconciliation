# File Storage - Agent Notes

## Common Gotchas
<!-- AI-UPDATED: 2025-01-04 - Multipart upload implementation -->
- S3 bucket must exist before file uploads work - LocalStack requires manual bucket creation
- Presigned URLs only work with S3 backend, not filesystem storage (returns None)
- File keys cannot start with '/' - S3 validation will reject them
- Multipart field names are case-sensitive: use "file", "file_name", "purpose" exactly
- MIME type detection requires file content analysis - cannot rely on filename extensions
- File size validation (10MB limit) happens after streaming, not before upload starts

## Design Decisions
<!-- AI-UPDATED: 2025-01-07 - Account-based organization -->
- Files organized by account hierarchy: `{merchant_id}/{profile_id}/{account_id}_{timestamp}/{filename}`
- Upload endpoint moved to account routes: `/accounts/{account_id}/upload_file`
- Account validation ensures file belongs to authenticated user's profile
- Timestamp combined with account_id for unique file organization
- Base64 encoding used for JSON responses to handle binary data
- File storage utils integrated into account utils for domain cohesion

## LocalStack Configuration
<!-- AI-UPDATED: 2025-01-04 - Development setup -->
- Endpoint URL: http://localhost:4566 for LocalStack
- Path style must be enabled: `path_style = true` in config
- Bucket name: hyperswitch-recon-files (must be created manually)
- AWS credentials can be dummy values for LocalStack: test/test

## When Working Here
<!-- AI-UPDATED: 2025-01-04 - Implementation patterns -->
- Always validate file uploads in utils before core processing
- Use streaming for multipart data to handle large files efficiently
- Check file existence before generating presigned URLs
- Extract filename from file_key using split('/').last() for headers
- MIME detection uses `infer` crate for content-based detection, not filename

## API Patterns
<!-- AI-UPDATED: 2025-01-07 - Account-based upload with core validation -->
- Upload endpoint integrated into account routes: `/accounts/{account_id}/upload_file`
- Multipart processing happens in utils/account.rs (not separate file_storage utils)
- File validation happens in core/account.rs for domain logic separation
- Route only processes multipart data, core handles validation and upload
- Error context includes account_id and file_key for debugging failed operations
- JWT authentication validates account access through profile_id
- Proper AccountError types thrown directly from utils (no error mapping)

## Security Considerations
<!-- AI-UPDATED: 2025-01-04 - File validation -->
- File name validation prevents path traversal: no "..", "/", "\"
- File size limits enforced to prevent quota exhaustion
- Presigned URLs provide temporary access without exposing credentials
- MIME type validation based on content, not user-provided headers
- Purpose-based file organization for access control patterns
