//! Warm-path, PCM-eviction, and library-reconciliation tests. Decode-ownership
//! lifecycle tests live in the sibling `tests` module.

use super::test_support::*;
use super::*;

#[test]
fn warm_play_sound_sets_playing_immediately() {
    let mut app = app_with_audio();
    let snd = sound("wav1");
    app.sounds = vec![snd];
    cache_pcm(&mut app, "wav1");

    // Warm cache hits claim the highlight synchronously via start_playback;
    // cold misses claim it optimistically in request_play (both #111).
    let _ = app.update(Message::PlaySound("wav1".into()));

    assert_eq!(app.playing(), Some("wav1"));
}

#[test]
fn warm_shortcut_activation_sets_playing_immediately() {
    let mut app = app_with_audio();
    let snd = sound("wav1");
    app.slots.set(0, snd.path.clone());
    app.sounds = vec![snd];
    cache_pcm(&mut app, "wav1");

    let _ = app.update(Message::ShortcutActivated(0));

    assert_eq!(app.playing(), Some("wav1"));
}

#[test]
fn pcm_eviction_removes_matching_waveform_envelope() {
    let mut app = app_with_audio();
    app.audio_store = crate::audio::AudioStore::new(32);
    let a = sound("a");
    let b = sound("b");
    app.sounds = vec![a.clone(), b.clone()];

    let _ = app.request_play(&a, false);
    let a_generation = app.play_generation;
    let _ = app.handle_decoded("a".into(), Ok(pcm(4)), dispatch(&app, a_generation));
    assert!(app.now_playing.envelope("a").is_some());

    let _ = app.request_play(&b, false);
    let b_generation = app.play_generation;
    let _ = app.handle_decoded("b".into(), Ok(pcm(8)), dispatch(&app, b_generation));

    assert!(app.audio_store.get_pcm("a").is_none());
    assert!(
        app.now_playing.envelope("a").is_none(),
        "waveform envelope must be evicted with its PCM victim"
    );
}

#[test]
fn pcm_eviction_keeps_active_waveform_envelope() {
    let mut app = app_with_audio();
    app.audio_store = crate::audio::AudioStore::new(32);
    app.sounds = vec![sound("a"), sound("b")];
    let a = app.sounds[0].clone();
    let b = app.sounds[1].clone();

    // Two cold presses in concurrent mode: B is the newest press and its
    // decode lands first, so B owns the now-playing UI.
    let _ = app.request_play(&a, false);
    let _ = app.request_play(&b, false);
    let _ = app.handle_decoded("b".into(), Ok(pcm(4)), dispatch(&app, 2));
    assert!(app.now_playing.envelope("b").is_some());

    // The late older decode for A lands and evicts B's PCM under cache
    // pressure while B is still the active now-playing sound.
    let _ = app.handle_decoded("a".into(), Ok(pcm(8)), dispatch(&app, 1));

    assert!(
        app.audio_store.get_pcm("b").is_none(),
        "B's PCM is the eviction victim"
    );
    assert!(
        app.now_playing.envelope("b").is_some(),
        "the active sound's waveform envelope must not be evicted mid-play"
    );
}

#[test]
fn library_reconcile_clears_playing_sound_removed_from_library() {
    let mut app = app_with_audio();
    app.sounds = vec![sound("gone")];
    start_now_playing(&mut app, "gone");

    app.sounds.clear();
    app.reconcile_playback_with_library();

    assert_eq!(app.playing(), None);
    assert!(!app.now_playing.has_playhead());
}

#[test]
fn reconcile_stops_engine_voice_for_removed_playing_sound() {
    let mut app = app_with_audio();
    let snd = sound("gone");
    app.sounds = vec![snd.clone()];
    // Warm play so a real engine voice exists, then drop the sound from the
    // library and reconcile: the orphaned voice must be stopped, not left to
    // honk to completion with no UI.
    cache_pcm(&mut app, "gone");
    let _ = app.request_play(&snd, false);
    let voice = app.play_generation;
    assert_eq!(app.playing(), Some("gone"));

    app.sounds.clear();
    app.reconcile_playback_with_library();

    assert_eq!(app.playing(), None);
    assert!(
        stopped_voices(&app).contains(&voice),
        "reconcile must stop the removed playing sound's engine voice"
    );
}
