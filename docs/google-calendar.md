# Google Calendar

The Today screen shows upcoming events from the connected account's **primary calendar**, over the next seven days. Recurring events are expanded; cancelled events and invitations you declined are omitted. Timed events use the device's timezone; all-day dates remain calendar dates. Prepare opens the existing project selection and readiness flow with the event title. Join opens a Google-provided HTTPS video link; it does not start audio capture.

## One-time setup

1. Create or select a Google Cloud project and enable **Google Calendar API**.
2. Configure the OAuth consent screen (Google Auth Platform). For an external app in Testing, add your account as a test user.
3. Create an OAuth client with application type **Desktop app**. Web application credentials are not compatible with this loopback flow.
4. In KebabM: **Today → Connect calendar → One-time Google Cloud setup**. Enter the client ID and client secret, then Connect with Google.
5. Review the read-only Calendar permission in your browser. The browser returns to an ephemeral `127.0.0.1` callback; the application finishes storing tokens and loads the agenda.

An AI Studio API key cannot replace OAuth authorization. External testing-mode refresh tokens can expire after seven days for these scopes. Reconnect when requested. Distribution to other users may require Google's OAuth verification; no verified production OAuth client is bundled.

## Behavior and storage

- Scope: `https://www.googleapis.com/auth/calendar.events.readonly`. No event creation, editing, deletion, email sending or automatic meeting recording.
- Authorization code with PKCE S256, exact state validation, loopback listener, 3-minute sign-in timeout and cancellation.
- Access/refresh tokens and client credentials stay in the existing credential adapter. On macOS this is the user-requested private **plaintext** local file (0700 directory / 0600 file), without Keychain access. Windows/Linux keep their existing adapters. Tokens are never returned by Calendar status or MCP tools.
- Expired access tokens refresh automatically. Refresh errors are visible. Failed agenda refresh leaves the last rendered agenda visible with a stale notice.
- Disconnect removes this device's stored Calendar credentials and clears the visible agenda. It does not revoke the Google account grant; remove that separately in Google Account's third-party access settings if desired.
- MCP: `calendar_status`, `calendar_events`, `calendar_disconnect`. Interactive connection is in the app, so OAuth client secrets need not be pasted into tool messages.
- Calendar data is not automatically sent to the AI. Preparing and starting a meeting passes the selected event's title/time into that meeting's context.

## Validation boundary

Automated fixtures test setup, disconnect, stale results, preparation and narrow layouts. Native tests cover callback state/path checks, cancellation, fragmented callback input, token refresh preservation and event normalization. Real account authorization requires the user's Desktop OAuth client and consent; fixture success does not prove live Google authorization.

References: [Google native OAuth](https://developers.google.com/identity/protocols/oauth2/native-app), [Calendar events list](https://developers.google.com/workspace/calendar/api/v3/reference/events/list).
