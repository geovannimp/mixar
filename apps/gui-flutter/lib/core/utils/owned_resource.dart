/// A resource a notifier owns and disposes exactly once.
abstract interface class OwnedResource {
  void dispose();
}
