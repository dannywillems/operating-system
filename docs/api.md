# Personal Operating System - API Documentation

## Overview

The Personal OS API provides RESTful endpoints for managing boards, columns, cards, and tags. All API endpoints are prefixed with `/api`.

## Authentication

The API supports two authentication methods:

### 1. Session Cookies (Web UI)

After logging in via the web interface, a session cookie is set automatically.

### 2. Bearer Tokens (API Access)

For programmatic access, create an API token and include it in the `Authorization` header:

```
Authorization: Bearer <your-api-token>
```

## Endpoints

### Authentication

#### Register a new user

```
POST /api/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "securepassword123",
  "name": "John Doe"
}
```

Response:
```json
{
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "John Doe",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

#### Login

```
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "securepassword123"
}
```

Response: Sets a session cookie and returns user info.

#### Logout

```
POST /api/auth/logout
```

Requires authentication. Invalidates the current session.

#### Create API Token

```
POST /api/auth/tokens
Content-Type: application/json

{
  "name": "My API Token",
  "scope": "write",
  "expires_in_days": 30
}
```

Scopes: `read`, `write`, `admin`

Response:
```json
{
  "id": "uuid",
  "token": "the-actual-token-save-this",
  "name": "My API Token",
  "scope": "write",
  "expires_at": "2024-02-01T00:00:00Z"
}
```

#### List API Tokens

```
GET /api/auth/tokens
```

#### Revoke API Token

```
DELETE /api/auth/tokens/:token_id
```

### Boards

#### Create Board

```
POST /api/boards
Content-Type: application/json

{
  "name": "My Project",
  "description": "Optional description"
}
```

#### List Boards

```
GET /api/boards
```

Returns all boards the authenticated user has access to.

#### Get Board Details

```
GET /api/boards/:board_id
```

Returns board with all columns, cards, and tags.

#### Update Board

```
PUT /api/boards/:board_id
Content-Type: application/json

{
  "name": "Updated Name",
  "description": "Updated description"
}
```

Requires `editor` or `owner` role.

#### Delete Board

```
DELETE /api/boards/:board_id
```

Requires `owner` role.

#### Add Board Permission

```
POST /api/boards/:board_id/permissions
Content-Type: application/json

{
  "user_id": "uuid",
  "role": "editor"
}
```

Roles: `reader`, `editor` (cannot add `owner`)

Requires `owner` role.

#### Remove Board Permission

```
DELETE /api/boards/:board_id/permissions/:user_id
```

Requires `owner` role. Cannot remove owner permission.

### Columns

#### Create Column

```
POST /api/boards/:board_id/columns
Content-Type: application/json

{
  "name": "To Do",
  "position": 0
}
```

Position is optional; if omitted, column is added at the end.

#### List Columns

```
GET /api/boards/:board_id/columns
```

#### Update Column

```
PUT /api/columns/:column_id
Content-Type: application/json

{
  "name": "In Progress"
}
```

#### Delete Column

```
DELETE /api/columns/:column_id
```

#### Move Column

```
PATCH /api/columns/:column_id/move
Content-Type: application/json

{
  "position": 2
}
```

### Cards

#### Create Card

```
POST /api/columns/:column_id/cards
Content-Type: application/json

{
  "title": "Task Title",
  "body": "Optional description",
  "visibility": "restricted",
  "start_date": "2024-01-15",
  "end_date": "2024-01-20",
  "due_date": "2024-01-18"
}
```

Visibility options:
- `private`: Only visible to editors and owners
- `restricted`: Visible to all board members (default)
- `public`: Visible to anyone (feature-flagged)

#### List Cards

```
GET /api/boards/:board_id/cards
```

Query parameters for filtering:
- `query`: Full-text search on title/body
- `tags`: Comma-separated list of tag UUIDs
- `start_date_from`, `start_date_to`: Filter by start date range
- `end_date_from`, `end_date_to`: Filter by end date range
- `due_date_from`, `due_date_to`: Filter by due date range
- `updated_from`, `updated_to`: Filter by last updated timestamp

Example:
```
GET /api/boards/:board_id/cards?query=bug&due_date_to=2024-01-31
```

#### Get Card

```
GET /api/cards/:card_id
```

#### Update Card

```
PUT /api/cards/:card_id
Content-Type: application/json

{
  "title": "Updated Title",
  "body": "Updated description",
  "visibility": "private",
  "due_date": "2024-01-25"
}
```

#### Delete Card

```
DELETE /api/cards/:card_id
```

#### Move Card

```
PATCH /api/cards/:card_id/move
Content-Type: application/json

{
  "column_id": "target-column-uuid",
  "position": 0
}
```

Moves a card to a different column and/or position. The target column must belong to the same board.

### Tags

#### Create Tag

```
POST /api/boards/:board_id/tags
Content-Type: application/json

{
  "name": "Bug",
  "color": "#dc3545"
}
```

Color defaults to `#6c757d` if not specified.

#### List Tags

```
GET /api/boards/:board_id/tags
```

#### Update Tag

```
PUT /api/tags/:tag_id
Content-Type: application/json

{
  "name": "Critical Bug",
  "color": "#ff0000"
}
```

#### Delete Tag

```
DELETE /api/tags/:tag_id
```

#### Add Tag to Card

```
POST /api/cards/:card_id/tags/:tag_id
```

#### Remove Tag from Card

```
DELETE /api/cards/:card_id/tags/:tag_id
```

## Error Responses

All errors return JSON with an `error` field:

```json
{
  "error": "Error message"
}
```

HTTP Status Codes:
- `400 Bad Request`: Invalid input
- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Resource not found
- `422 Unprocessable Entity`: Validation error
- `500 Internal Server Error`: Server error

## Example Workflows

### Create a Kanban Board with Columns

```bash
# Create a board
curl -X POST http://localhost:3000/api/boards \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "Sprint 1"}'

# Create columns
curl -X POST http://localhost:3000/api/boards/$BOARD_ID/columns \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "To Do"}'

curl -X POST http://localhost:3000/api/boards/$BOARD_ID/columns \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "In Progress"}'

curl -X POST http://localhost:3000/api/boards/$BOARD_ID/columns \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "Done"}'
```

### Create and Move a Card

```bash
# Create a card in "To Do" column
curl -X POST http://localhost:3000/api/columns/$TODO_COLUMN_ID/cards \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title": "Implement login", "body": "Add user authentication"}'

# Move card to "In Progress" column
curl -X PATCH http://localhost:3000/api/cards/$CARD_ID/move \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"column_id": "'$IN_PROGRESS_COLUMN_ID'", "position": 0}'
```

### Add Tags to Cards

```bash
# Create a tag
curl -X POST http://localhost:3000/api/boards/$BOARD_ID/tags \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "Priority", "color": "#ffc107"}'

# Add tag to card
curl -X POST http://localhost:3000/api/cards/$CARD_ID/tags/$TAG_ID \
  -H "Authorization: Bearer $TOKEN"

# Filter cards by tag
curl "http://localhost:3000/api/boards/$BOARD_ID/cards?tags=$TAG_ID" \
  -H "Authorization: Bearer $TOKEN"
```
