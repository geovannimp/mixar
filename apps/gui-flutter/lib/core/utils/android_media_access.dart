import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform, kIsWeb;
import 'package:flutter/services.dart';

/// Channel hosted by `MainActivity` on Android. The host asks for the media
/// read permission at runtime; nothing else lives here. `MainActivity`'s
/// `MEDIA_ACCESS_CHANNEL` is the other half of the name, so a rename has to
/// touch both sides.
const androidMediaAccessChannel = 'top.mixar.app/media_access';

/// Method `MainActivity` answers on [androidMediaAccessChannel].
const androidEnsureAudioReadMethod = 'ensureAudioReadPermission';

const _channel = MethodChannel(androidMediaAccessChannel);

/// Shown when the runtime permission is refused, so the folder import stops
/// with a reason instead of landing as an empty collection.
const audioReadDeniedMessage =
    'Mixar needs access to your audio files to read a folder. Allow music '
    'access for Mixar in system settings, then try again.';

/// Whether the app may read audio files at their on-disk path.
///
/// Android scoped storage hides other apps' media from the raw `std::fs` reads
/// the library scanner does until the process holds the media read permission,
/// so a folder import without it would add a collection that lists nothing.
/// Every other platform reads paths directly, with no permission involved.
Future<bool> ensureAudioReadAccess() async {
  if (kIsWeb || defaultTargetPlatform != TargetPlatform.android) {
    return true;
  }
  try {
    return await _channel.invokeMethod<bool>(androidEnsureAudioReadMethod) ??
        false;
  } on Exception {
    // Any channel failure means no access: a refused call, or
    // `MissingPluginException` on a build without the host handler. Fail the
    // import rather than create a collection that can only be empty. Errors
    // that are not exceptions (a malformed host reply) still propagate; the
    // import actions surface those like any other failure.
    return false;
  }
}
