# Notification Systems — Multi-Channel Delivery, Preferences & Infrastructure

## Overview

Notification systems deliver messages to users across multiple channels (email, push, SMS, in-app, Slack, webhooks) with **preference management**, **delivery optimization**, and **infrastructure scalability**. A notification is a fact ("user received a message") that must reach the user reliably, respecting their chosen channels and frequency preferences.

## Architecture Layers

### 1. Trigger / Event Source

An event triggers notification generation:
- User action → "User mentioned @alice in a comment"
- System event → "Your subscription renews in 3 days"
- Schedule → "Daily digest of unread messages"
- External → Third-party webhook

Events flow into a queue (Kafka, RabbitMQ, SQS) for reliable processing.

### 2. Notification Center

The notification center is the **single source of truth** for user notification preferences and history.

**Schema:**

```
notifications:
  id: uuid
  user_id: int
  title: string
  body: string
  category: enum ("message", "billing", "mention", "update")
  timestamp: datetime
  read: boolean
  archived: boolean
  
user_preferences:
  user_id: int
  category: string
  channels: set ["email", "push", "sms", "in_app", "slack"]
  frequency: enum ["instant", "daily", "weekly", "off"]
  quiet_hours: ?timerange  (e.g., 10pm-8am)
```

**Notification center services:**
- **Deliver:** Route notification to enabled channels, respecting preferences
- **Store:** Persist notification for user's in-app notification center
- **Query:** List user's notifications (paginated, filtered by category/read)
- **Preferences:** Get/set user's channels and frequency

### 3. Preference Management

Users choose how to receive notifications. Preferences are **rich and multi-layered**:

| Scope | Example |
| --- | --- |
| Global | "I only want critical alerts" |
| Category | "Messages: email + push; Billing: email only; Updates: off" |
| Channel | "Email: yes; SMS: no (too expensive)" |
| Time | "No push notifications between 10pm-8am" |
| Frequency | "Don't send me more than 1 email per day" |
| Unsubscribe | One-click opt-out from email footer |

**Preference storage:** Normalized table. Don't duplicate preferences in event queue.

**Preference lookup:** Cache in Redis. On each notification, query cache for user's preferences; update cache when user changes preferences.

**Defaults:** New users default to "instant, all channels" or conservative defaults (email only). Aggressive defaults lead to unsubscribes.

### 4. Delivery Layer

**Email:**
- Use transactional email service (SendGrid, Mailgun, Amazon SES). Never send from own servers; triggers spam filtering.
- Signed with DKIM/SPF to avoid spoofing. Service provides reputation management.
- Cost: ~$0.0001 per email at scale

**Push notifications (mobile):**
- iOS: Apple Push Notification service (APNs). Requires provisioning profile + certificates.
- Android: Firebase Cloud Messaging (FCM)
- Web: Web Push API (service worker-based)
- Cost: Free (you host it) or ~$0.0005 per notification (Expo, Firebase)

**SMS:**
- Twilio, AWS SNS, Vonage. Heavily rate-limited (users tolerate email spam less than SMS spam).
- Cost: ~$0.01 per SMS

**In-app:**
- Display notification center. User sees all notifications in one place (not spread across email/push).
- Can be delivered in-app (real-time WebSocket) or fetched on app open.

**Slack / integrations:**
- Send message to user's Slack workspace (if user linked account). Authenticate with OAuth.
- Cost: Free (Slack API)

**Webhook deliveries:**
- For each channel, attempt delivery with retries (exponential backoff). Track success/failure.
- Implement idempotency: if sending email and network fails mid-send, resend same email with same ID.

### 5. Batching & Digesting

**Problem:** A user receives 50 notifications over 4 hours. Sending 50 separate emails = spam.

**Solution:** Batch related notifications into a digest.

```
Morning digest (9 AM):
- 5 new messages from coworkers
- 12 mentions in discussions
- 1 billing alert
→ Send 1 email instead of 18
```

**Digest strategies:**
- **Time-window:** Collect notifications over 1 hour, send as digest
- **Count-based:** Collect until 5 notifications, send immediately
- **Event-type:** Group by category (all mentions together, all billing together)
- **User preference:** User chooses frequency ("send me a daily digest")

**Tradeoffs:** Digests reduce email volume but increase latency. A user gets mention notification 1 hour after mention, not instantly.

**Frequency caps:** Don't send more than N notifications per day/week. Low-priority digests wait for quiet period.

## Template Management

Notifications need **consistent, branded content**. Use a template engine.

**Template schema:**

```
template:
  id: string
  name: string
  category: string
  channels: set ["email", "push", "sms"]
  body_template: string  (Handlebars, Jinja2, etc)
  subject_template: string  (email subject)
  example_data: object
```

**Example template:**

```
Name: message_received
Subject: New message from {{ sender_name }}
Body: 
  {{ sender_name }} sent you a message:
  "{{ message_preview }}"
  
  Reply: {{ action_url }}
```

**Rendering:** On delivery, substitute `{{ }}` placeholders with actual data. Data validation prevents injection attacks.

**A/B testing:** Test multiple subject lines; track open rates. Choose winner.

## Delivery Optimization & Reliability

### Idempotency

Network failures happen: database timeout, email service temporarily down, cloud function crashes. **Idempotency** ensures repeat deliveries don't cause duplicates.

Assign each notification a **delivery ID** (UUID). On each attempt, include delivery_id. Service stores mapping:

```
delivery_id → notification_id, delivery_result
```

If same delivery_id arrives again, return cached result ("already delivered") not duplicate.

### Retry strategy

Failed delivery (email service timeout, temporary SMS gateway error) should retry. Exponential backoff:

```
Attempt 1: immediate
Attempt 2: 1 minute later
Attempt 3: 10 minutes
Attempt 4: 1 hour
Attempt 5: 1 day (give up if >5 attempts)
```

Track failure reason: permanent errors (invalid email, user unsubscribed) vs transient (service timeout, rate limit).

### Quiet hours

Don't disturb users at 3 AM. Respect quiet_hours preference. If notification arrives during quiet hours, queue it for next available slot (morning, lunch, evening).

### Deduplication

Same event triggers multiple notifications inadvertently. Example: user deleted a comment → "comment deleted" notification AND "spam reported" notification both send separately. Deduplicate by grouping notifications from same event within short time window.

## Unsubscribe & Preference Management UI

**Legal requirement** (CAN-SPAM in US, GDPR in EU): Email must have one-click unsubscribe. Use RFC 8058 (`List-Unsubscribe` header) to enable unsubscribe button in email client.

**In-app preference center:** Let users granularly control channels per category, frequency, quiet hours. Don't force all-or-nothing unsubscribe.

**Unsubscribe tracking:** User clicks unsubscribe → set preference to "off" for that category/channel. Don't immediately delete; keep record for compliance (CAN-SPAM, CAP, GDPR).

## Notification Infrastructure

**High-level flow:**

```
Event Source (user action, cron) 
  → Message Queue (Kafka, SQS)
  → Notification Worker (read event, look up preferences)
  → Delivery Layer (email, push, SMS, in-app, Slack)
  → Notification Center (store for in-app history)
  → Analytics (track delivery success, engagement)
```

**Scalability:**

- **Message queue:** Decouple event source from delivery. Allows burst absorbtion. Store 1M+ pending events.
- **Parallel workers:** Multiple machines process queue in parallel. Each worker processes 1000s of notifications per second.
- **Rate limiting:** Prevent notification storms. Cap 100 notifications per user per day; remainder queued or dropped.

**Reliability:**

- **At-least-once delivery:** Messages stay in queue until acknowledged (processed successfully). Retries on failure.
- **Dead-letter queue:** Messages that fail 5+ times go to DLQ for manual investigation.
- **Monitoring:** Track delivery latency (time from event to first delivery), success rate (% delivered), engagement (open rate for email, click rate for push).

## Privacy & Compliance

**GDPR:** User has right to delete their data. Delete notifications, preferences, and delivery history (except legal holds).

**CAN-SPAM (US email law):** Email must include:
- Sender identity (From: must be real)
- Subject line must not be deceptive
- Clear unsubscribe link
- Physical mailing address

**Preference portability:** User can export their notification history and preferences in machine-readable format (JSON, CSV).

**Data minimization:** Don't store more notification history than needed (30 days for transactional events, 1 year for important alerts).

## Typical Technology Stack

| Component | Examples |
| --- | --- |
| Event source | Kafka, RabbitMQ, AWS SQS, Google Pub/Sub |
| Notification center DB | PostgreSQL, MongoDB, Firebase |
| Preferences cache | Redis |
| Email delivery | SendGrid, Mailgun, AWS SES |
| Push notifications | Firebase Cloud Messaging, APNs |
| SMS | Twilio, AWS SNS |
| Template engine | Handlebars, Jinja2, Nunjucks |
| Orchestration | Apache Airflow (scheduled digests), custom workers (Celery, Go) |
| Analytics | Mixpanel, Segment, custom logging |

## Common Pitfalls

**Ignoring preferences:** Sending push at 3 AM because user subscribed to category. Always check quiet_hours.

**No deduplication:** Same event triggers 3 notifications → user receives 3 emails. Group and deduplicate.

**Slow delivery:** Email arrives 1 hour after event. Use async workers + message queue for sub-second latency.

**Broken unsubscribe:** User can't opt-out → legal risk. Test unsubscribe link works.

**No idempotency:** Network failure → duplicate emails sent. Implement delivery_id tracking.

## See Also

- [api-webhooks.md](api-webhooks.md) — Webhook delivery patterns, retries, idempotency
- [mobile-push-notifications.md](mobile-push-notifications.md) — APNs, FCM detailed architecture
- [data-engineering-streaming.md](data-engineering-streaming.md) — Stream processing for real-time events
- [distributed-replication.md](distributed-replication.md) — Consistency and reliability at scale
- [security-authentication-attacks.md](security-authentication-attacks.md) — Phishing risks in notification content