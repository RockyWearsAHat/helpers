# Mobile Push Notifications — APNs, FCM, Rich Notifications, Channels & Delivery Optimization

## Overview

Push notifications are server-initiated messages delivered to users' devices. They enable real-time engagement (messages, alerts, reminders) without the app being open. Two major systems: Apple Push Notification service (APNs) for iOS, Firebase Cloud Messaging (FCM) for Android. Delivery is best-effort, not guaranteed; network interruption or user-disabled notifications prevent delivery.

## APNs (Apple Push Notification Service)

Apple's proprietary system for iOS, macOS, watchOS.

### Architecture

```
[Provider Server] 
         ↓ (HTTPS with TLS cert)
[APNs Gateway] 
         ↓ (proprietary protocol)
[Apple Device]
```

Providers connect to APNs Gateway (api.push.apple.com:443) with a certificate or token. Sends notifications with device tokens. APNs queues and delivers.

### Token-Based Authentication (Modern)

Providers generate a JWT signed with a private key. Better than certificate management (no renewal overhead).

```
1. Provider generates JWT from private key (valid 1 hour)
2. Sends JWT in Authorization header: Bearer <jwt>
3. APNs validates JWT signature
```

**Advantages**: No certificate renewal cycles, key rotation is simpler.

### Certificate-Based Authentication (Legacy)

Providers authenticate with a TLS client certificate (obtained from Apple Developer portal).

**Process**:

1. Export certificate + private key from Apple Developer portal (PKCS#12 format).
2. Convert to PEM.
3. Establish mTLS connection to APNs: provider cert validates to Apple server cert.

**Disadvantage**: Certificates expire annually, require renewal and re-deployment.

### Device Token Lifecycle

Each device registers with APNs when app installs, receives a unique token (64 hex chars).

```swift
import UserNotifications

UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { granted, error in
  DispatchQueue.main.async {
    if granted {
      UIApplication.shared.registerForRemoteNotifications()
    }
  }
}

// Called when device receives token
func application(_ application: UIApplication, 
                 didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
  let token = deviceToken.map { String(format: "%02.2hhx", $0) }.joined()
  print("APNs token: \(token)") // Send to provider server
}
```

**Token Refresh**: APNs invalidates tokens periodically (especially after OS updates). Users should re-register. Provider server should handle token expiration gracefully (failed deliveries marked invalid).

### Notification Format

APNs accepts JSON:

```json
{
  "aps": {
    "alert": {
      "title": "New Message",
      "body": "You have a message from Alice"
    },
    "badge": 1,
    "sound": "default",
    "category": "MESSAGE"
  },
  "custom_data": { "sender_id": 123 }
}
```

**Fields**:

- **alert**: Title + body. If omitted, silent push.
- **badge**: App icon badge number.
- **sound**: Sound file name (built-in or custom).
- **category**: Groups notifications for grouped display or custom actions.
- **mutable-content**: 1 = allows notification service extension to modify content before display.
- **custom_data**: App-specific metadata (not displayed, accessed by notification handler).

### Priority & Delivery

**Priority**:

- `10` (default): Delivers immediately or when device connected.
- `1`: Low priority, deferred. Useful for non-urgent notifications (digest emails).

```
POST /3/device/<token>
apns-priority: 1
// Low priority: can be delayed hours
```

**Expiration**: Notifications with `apns-expiration` header are discarded if not delivered by timestamp. Default: 1 hour.

### Delivery Rate & Failures

APNs does not guarantee delivery. Failures:

- Device offline → queued (respects expiration). If offline past expiration, dropped.
- Device unreachable (sim removed, switched networks) → dropped after queue timeout.
- Invalid token → marked invalid, provider should remove token.

**HTTP Status Codes**:

- `200`: Accepted for delivery (not guaranteed delivered).
- `400 BadDeviceToken`: Token invalid, remove it.
- `410 Unregistered`: Token no longer valid.
- `429 TooManyRequests`: Rate limited, backoff.
- `500`: Temporary server error, retry.

## FCM (Firebase Cloud Messaging)

Google's cross-platform messaging service (Android, iOS, web). Owned by Firebase (Google).

### Architecture

```
[Provider Server]
      ↓ (HTTP v1 API, OAuth 2.0)
[FCM Backend]
      ↓ (proprietary protocol)
[Device with Google Play Services]
```

FCM handles multiplexing: single connection to Google Play Services receives messages for all apps on device.

### Authentication

FCM uses OAuth 2.0 service account:

```
1. Provider claims a service account credential (JSON key file from Firebase console).
2. Exchanges credential for short-lived OAuth token.
3. Includes token in Authorization: Bearer header.
```

**Compared to APNs**: Simpler (no certificate management), but depends on Google infrastructure/OAuth.

### Device Registration

FCM generates a device registration token (Instance ID token) when app starts.

```kotlin
// Android
FirebaseMessaging.getInstance().token.addOnCompleteListener { task ->
  if (task.isSuccessful) {
    val token = task.result
    Log.d(TAG, "FCM token: $token")
    // Send to provider server
  }
}
```

**Token Refresh**: FCM may invalidate tokens (rare). App should listen for token refresh events and send new token to server.

```kotlin
class MyFirebaseMessagingService : FirebaseMessagingService() {
  override fun onNewToken(token: String) {
    Log.d(TAG, "New FCM token: $token")
    sendTokenToServer(token)
  }
}
```

### Message Types

**Data Message**: Raw key-value pairs, app handles processing (no notification shown unless app does it).

```json
{
  "data": {
    "title": "New Message",
    "sender_id": "123"
  }
}
```

**Notification Message**: FCM displays notification automatically; app can handle it in foreground.

```json
{
  "notification": {
    "title": "New Message",
    "body": "From Alice"
  },
  "data": {
    "sender_id": "123"
  }
}
```

**Topic Messages**: Send to all devices subscribed to a topic (not individual tokens).

```kotlin
FirebaseMessaging.getInstance().subscribeToTopic("weather")
// Server sends to /topics/weather instead of individual tokens
```

**Device Groups**: Send to multiple tokens at once (deprecated in favor of topics).

### Notification Channels (Android)

Android 8+ requires notifications be assigned to a channel. Channels group notifications by type, allowing users to control per-channel behavior.

```kotlin
if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
  val channel = NotificationChannel(
    "messages",
    "Messages",
    NotificationManager.IMPORTANCE_DEFAULT
  )
  channel.description = "Incoming chat messages"
  val manager = context.getSystemService(NotificationManager::class.java)
  manager.createNotificationChannel(channel)
}

// When sending notification, assign channel
val builder = NotificationCompat.Builder(context, "messages")
  .setSmallIcon(R.drawable.ic_notify)
  .setContentTitle("New Message")
```

**Importance Levels**:

- `IMPORTANCE_NONE` (0): No notification shown, silent.
- `IMPORTANCE_MIN` (1): Notification in notification drawer, no sound.
- `IMPORTANCE_LOW` (2): Notification + sound off.
- `IMPORTANCE_DEFAULT` (3): Notification + default sound.
- `IMPORTANCE_HIGH` (4): Notification + sound + heads-up display.

**iOS Equivalent**: Sounds, Critical alerts, Provisional (silent unless unlocked).

### Priority Levels (FCM)

- **High**: Attempt immediate delivery. Wakes device temporarily.
- **Normal**: Delivery deferred to save battery. Typical for non-urgent updates.

```json
{
  "message": {
    "token": "...",
    "webpush": {
      "fcmOptions": {
        "analyticsLabel": "important_message"
      }
    },
    "android": {
      "priority": "high"
    }
  }
}
```

## Rich Notifications

Both platforms allow multimedia: images, custom actions, sound.

### Notification Service Extension (iOS)

Runs in a separate process before notification is displayed, allowing modification.

```swift
class NotificationService: UNNotificationServiceExtension {
  override func didReceive(_ request: UNNotificationRequest, 
                         withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void) {
    let mutableContent = request.content.mutableCopy() as! UNMutableNotificationContent
    
    // Fetch image and attach
    if let imageUrl = mutableContent.userInfo["image_url"] as? String {
      if let image = UIImage(contentsOfFile: imageUrl) {
        let attachment = try? UNNotificationAttachment(identifier: "image", url: URL(fileURLWithPath: imageUrl), options: nil)
        if let attachment = attachment {
          mutableContent.attachments = [attachment]
        }
      }
    }
    
    contentHandler(mutableContent)
  }
}
```

**Use Cases**: Download message preview image, decrypt notification body, add local context.

**Constraint**: 30-second timeout. If extension takes too long, original notification displayed.

### Notification Actions (iOS)

Custom buttons on notification (reply, snooze, dismiss).

```swift
let replyAction = UNTextInputNotificationAction(
  identifier: "REPLY",
  title: "Reply",
  options: .authenticationRequired
)
let category = UNNotificationCategory(
  identifier: "MESSAGE",
  actions: [replyAction],
  intentIdentifiers: [],
  options: []
)
UNUserNotificationCenter.current().setNotificationCategories([category])

// In notification, set category to "MESSAGE"
```

When user taps reply, app receives callback:

```swift
func userNotificationCenter(_ center: UNUserNotificationCenter,
                          didReceive response: UNNotificationResponse,
                          withCompletionHandler completionHandler: @escaping () -> Void) {
  if let textResponse = response as? UNTextInputNotificationResponse {
    let replyText = textResponse.userText
    // Handle reply
  }
  completionHandler()
}
```

### Rich Notifications (Android)

Notifications display images via BigPictureStyle:

```kotlin
val bitmap = BitmapFactory.decodeFile(imagePath)
NotificationCompat.Builder(context, "messages")
  .setSmallIcon(R.drawable.ic_notify)
  .setStyle(NotificationCompat.BigPictureStyle()
    .bigPicture(bitmap)
    .setBigContentTitle("New Message"))
  .build()
```

**Custom Layouts (RemoteViews)**: Mostly deprecated in favor of system styles (simple, reliable across devices).

## Silent Push Notifications

Background updates without user notification, used for data sync.

**iOS**: `"content-available": 1` (silent alert-less push).

```json
{
  "aps": {
    "content-available": 1,
    "custom_data": { "new_posts": 5 }
  }
}
```

App receives in `application:didReceiveRemoteNotification:fetchCompletionHandler:` and can fetch new data.

**Constraints**: OS limits background executions (apps get ~30 seconds, rate-limited to ~1 per 10 minutes).

**Android**: `priority: high` for data messages; app's `onMessageReceived()` is called in background.

```kotlin
override fun onMessageReceived(remoteMessage: RemoteMessage) {
  if (remoteMessage.data.isNotEmpty()) {
    // Sync silently
    syncData()
  }
}
```

## Background Processing & Delivery

Notifications arriving while app backgrounded can trigger background work.

**Android WorkManager**: Schedule work to run soon, respecting device battery/standby state.

```kotlin
// In response to notification, enqueue work
WorkManager.getInstance(context).enqueueUniqueWork(
  "notify_work",
  ExistingWorkPolicy.REPLACE,
  OneTimeWorkRequestBuilder<NotificationWorker>().build()
)
```

**iOS BackgroundTasks**: app can request long-running task, OS grants 30 seconds if conditions met (plugged in, good network).

```swift
BGTaskScheduler.shared.submit(BGProcessingTaskRequest(identifier: "com.example.sync")) { error in
  if error != nil {
    // Failed to schedule
  }
}
```

## iOS vs Android Differences

| Aspect | iOS | Android |
|--------|-----|---------|
| Authentication | Certificate or JWT token | OAuth 2.0 service account |
| Token | 64-char hex string | Long alphanumeric string |
| Rate limiting | Per-token throttling if too many failures | Per-provider rate limits |
| Notification Channels | User-level (iOS 12+) | Required per notification type (Android 8+) |
| Silent Push | content-available | High-priority data message |
| Action Buttons | No, UNNotificationAction | No, RemoteInput in Android 4.1+ |
| Delivery Guarantee | Best-effort, long queueing | Best-effort, shorter queueing |

## Delivery Rate Optimization

**Best Practices**:

1. **Validate tokens at receive time**: Track failures, remove invalid tokens.
2. **Batch sends**: Send thousands of notifications together (more efficient than staggered).
3. **Use high priority sparingly**: Too many high-priority notifications cause OS to deprioritize.
4. **Respect opt-out**: User must be able to disable notifications entirely (Settings on device, in-app toggle).
5. **Monitor delivery**: Log which notifications were sent, which failed. Use Firebase Analytics to measure engagement.

## See Also

- [Mobile Development Patterns](mobile-development-patterns.md) — app lifecycle, background interruption
- [API Authentication](api-authentication.md) — token/certificate management patterns
- [Cryptography: Key Management](cryptography-key-management.md) — securing provider keys
- [Mobile Security](security-mobile-security.md) — validating notification content safety