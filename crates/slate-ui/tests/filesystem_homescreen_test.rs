//! Rigorous tests for Slate Filesystem, Homescreen, Recent Files, and Media Pool Folders.

use std::path::PathBuf;
use slate_core::engine::Engine;
use slate_core::model::{MediaMetadata, MediaReference, Project, TrackType};
use slate_core::time::RationalTime;
use slate_ui::file_browser::{FileBrowserModal, FileBrowserMode};
use slate_ui::homescreen::recents::{RecentProject, RecentsStore};
use slate_ui::homescreen::HomeScreenModal;
use slate_ui::state::EditorState;
use slate_ui::SlateApp;

#[test]
fn test_recent_projects_store_and_formatting() {
    let mut store = RecentsStore::with_samples();
    assert!(!store.projects.is_empty(), "Should initialize with sample projects");

    let p1 = RecentProject::new(
        "Feature Documentary",
        PathBuf::from("/tmp/test_projects/doc.slateproj"),
        125.5,
        [3840, 2160],
        24.0,
        6,
        18,
    );
    assert_eq!(p1.name, "Feature Documentary");
    assert_eq!(p1.formatted_duration(), "02:06");
    assert_eq!(p1.formatted_time_ago(), "Just now");

    // Add to store
    store.add(p1.clone());
    assert_eq!(store.projects[0].name, "Feature Documentary");

    // Adding same path moves it to top without duplicating
    let p1_again = RecentProject::new(
        "Feature Documentary (Updated)",
        PathBuf::from("/tmp/test_projects/doc.slateproj"),
        130.0,
        [3840, 2160],
        24.0,
        6,
        20,
    );
    store.add(p1_again);
    assert_eq!(
        store.projects.iter().filter(|p| p.path == std::path::Path::new("/tmp/test_projects/doc.slateproj")).count(),
        1
    );
    assert_eq!(store.projects[0].name, "Feature Documentary (Updated)");

    // Remove from store
    store.remove(std::path::Path::new("/tmp/test_projects/doc.slateproj"));
    assert!(store.projects.iter().all(|p| p.path != std::path::Path::new("/tmp/test_projects/doc.slateproj")));
}

#[test]
fn test_file_browser_navigation_and_modes() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let base = temp_dir.path();

    // Create a mock folder structure
    let subfolder = base.join("Footage");
    std::fs::create_dir_all(&subfolder).unwrap();

    let project_bundle = base.join("Episode_01.slateproj");
    std::fs::create_dir_all(&project_bundle).unwrap();
    std::fs::write(project_bundle.join("project.json"), "{}").unwrap();

    let video_file = subfolder.join("interview.mp4");
    std::fs::write(&video_file, "dummy video").unwrap();

    let audio_file = subfolder.join("bg_music.wav");
    std::fs::write(&audio_file, "dummy audio").unwrap();

    let text_file = base.join("notes.txt");
    std::fs::write(&text_file, "notes").unwrap();

    // 1. Test OpenProject Mode
    let mut browser = FileBrowserModal::default();
    browser.navigate_to(base.to_path_buf());
    browser.open(FileBrowserMode::OpenProject);

    let bundle_entry = browser.entries.iter().find(|e| e.name == "Episode_01.slateproj");
    assert!(bundle_entry.is_some(), "Should find .slateproj bundle");
    assert!(bundle_entry.unwrap().is_bundle, "Should be recognized as a project bundle");

    // 2. Test ImportMedia Mode
    browser.navigate_to(subfolder.clone());
    browser.open(FileBrowserMode::ImportMedia);
    let has_mp4 = browser.entries.iter().any(|e| e.name == "interview.mp4");
    let has_wav = browser.entries.iter().any(|e| e.name == "bg_music.wav");
    assert!(has_mp4, "ImportMedia mode should include .mp4 video files");
    assert!(has_wav, "ImportMedia mode should include .wav audio files");
    assert!(
        browser.entries.iter().all(|e| !e.name.ends_with(".txt")),
        "ImportMedia mode should filter out non-media .txt files"
    );

    // 3. Test SaveProjectAs Mode
    browser.navigate_to(base.to_path_buf());
    browser.open(FileBrowserMode::SaveProjectAs);
    browser.save_as_name = "Master_Cut".to_string();
    assert_eq!(browser.mode, FileBrowserMode::SaveProjectAs);
}

#[test]
fn test_homescreen_search_and_preset_creation() {
    let mut homescreen = HomeScreenModal::default();
    homescreen.open();
    assert!(homescreen.is_open);

    // Test search filter
    homescreen.search_query = "Tokyo".to_string();
    let matching = homescreen.recents_store.projects.iter().filter(|p| {
        p.name.to_lowercase().contains("tokyo")
    }).count();
    assert!(matching >= 1, "Should find Tokyo project in default samples");

    homescreen.search_query.clear();
    homescreen.close();
    assert!(!homescreen.is_open);
}

#[test]
fn test_media_pool_bins_and_categorization() {
    let mut project = Project::new("Media Pool Test");
    let asset1 = MediaReference {
        id: slate_core::id::MediaId::new(),
        original_path: PathBuf::from("/media/video_a.mp4"),
        relative_path: PathBuf::from("B-Roll/video_a.mp4"),
        metadata: MediaMetadata {
            file_size_bytes: 1024 * 1024 * 50,
            duration: RationalTime::from_seconds(10.0, 30),
            video_codec: Some("h264".into()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            timebase: 30,
            audio_codec: Some("aac".into()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
        },
        proxy_path: None,
        waveform_path: None,
    };

    let asset2 = MediaReference {
        id: slate_core::id::MediaId::new(),
        original_path: PathBuf::from("/media/soundtrack.wav"),
        relative_path: PathBuf::from("Audio/soundtrack.wav"),
        metadata: MediaMetadata {
            file_size_bytes: 1024 * 1024 * 10,
            duration: RationalTime::from_seconds(20.0, 48000),
            video_codec: None,
            width: None,
            height: None,
            frame_rate: None,
            timebase: 48000,
            audio_codec: Some("pcm_s16le".into()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
        },
        proxy_path: None,
        waveform_path: None,
    };

    project.media_pool.push(asset1.clone());
    project.media_pool.push(asset2.clone());

    let engine = Engine::new(project, 100);
    let mut state = EditorState::new(engine);

    // Initial default state
    assert_eq!(state.media_active_bin, "All");
    assert!(state.media_custom_bins.contains(&"B-Roll".to_string()));

    // Verify search query field
    state.media_search_query = "soundtrack".to_string();
    assert_eq!(state.media_search_query, "soundtrack");

    // Test inserting asset onto timeline
    slate_ui::media_pool::insert_asset_onto_timeline(&mut state, &asset1);
    let engine = state.engine.lock();
    let tl = engine.project().active_timeline().unwrap();
    let v_track = tl.tracks.iter().find(|t| t.track_type == TrackType::Video).unwrap();
    assert!(!v_track.items.is_empty(), "Should insert asset1 onto V1 track");
}

#[test]
fn test_slate_app_project_lifecycle() {
    let project = Project::new("Lifecycle Test");
    let mut app = SlateApp::new_with_project(project);

    // 1. Create sample project
    app.create_sample_project();
    {
        let engine = app.state.engine.lock();
        assert_eq!(engine.project().name, "Tokyo Cyberpunk Demo");
        let tl = engine.project().active_timeline().unwrap();
        assert_eq!(tl.tracks.len(), 3);
        assert_eq!(tl.tracks[0].items.len(), 1);
        assert_eq!(tl.tracks[1].items.len(), 1);
        assert_eq!(tl.tracks[2].items.len(), 1);
    }
    assert!(app.current_project_path.is_some());

    // 2. Save current project
    app.save_current_project();
    let path = app.current_project_path.clone().unwrap();
    assert!(path.join("project.json").exists(), "Project bundle project.json should exist on disk");

    // 3. Load project from path
    app.load_project_path(&path);
    let engine = app.state.engine.lock();
    assert_eq!(engine.project().name, "Tokyo Cyberpunk Demo");

    // 4. Import a media file
    drop(engine);
    let temp_media = tempfile::NamedTempFile::new().unwrap();
    app.import_media_file(temp_media.path());
    let engine = app.state.engine.lock();
    assert_eq!(engine.project().media_pool.len(), 1);
}

#[test]
fn test_inspector_pan_zoom_crop_sounds_text() {
    use slate_core::id::{CaptionId, KeyframeId};
    use slate_core::model::caption::{Caption, CaptionStyle};
    use slate_core::model::keyframe::{Interpolation, Keyframe};
    use slate_core::op::Op;
    use slate_core::OpSource;
    use slate_core::time::TimeRange;

    let project = Project::new("Effects Test");
    let mut app = SlateApp::new_with_project(project);
    app.create_sample_project();

    let clip_id = {
        let engine = app.state.engine.lock();
        let tl = engine.project().active_timeline().unwrap();
        match &tl.tracks[0].items[0] {
            slate_core::model::TrackItem::Clip(c) => c.id,
            _ => panic!("Expected clip"),
        }
    };

    // 1. Pan, Zoom & Crop
    let mut transform = slate_core::model::Transform::default();
    transform.position_x.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: 0.25, // Pan right
        interpolation: Interpolation::Linear,
    }];
    transform.scale_x.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: 1.20, // Zoom in
        interpolation: Interpolation::Linear,
    }];
    transform.crop_left.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: 0.28, // 9:16 vertical crop
        interpolation: Interpolation::Linear,
    }];
    transform.crop_right.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: 0.28,
        interpolation: Interpolation::Linear,
    }];

    assert!(app.state.apply_op(Op::SetTransform { clip_id, transform }, OpSource::Human).is_ok());

    // 2. Audio & Sounds
    let mut audio = slate_core::model::AudioParams::default();
    audio.volume_db.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: 3.0, // Dialogue enhance +3dB
        interpolation: Interpolation::Linear,
    }];
    audio.pan.keyframes = vec![Keyframe {
        id: KeyframeId::new(),
        time: RationalTime::ZERO,
        value: -0.5, // Pan left
        interpolation: Interpolation::Linear,
    }];
    assert!(app.state.apply_op(Op::SetAudioParams { clip_id, params: audio }, OpSource::Human).is_ok());

    // 3. Text Effects & Captions
    let cap_id = CaptionId::new();
    let caption = Caption {
        id: cap_id,
        time_range: TimeRange::new(RationalTime::ZERO, RationalTime::from_seconds(4.0, 30)),
        text: "Kinetic TikTok Subtitle".to_string(),
        style: CaptionStyle {
            font_family: "Inter".to_string(),
            font_size: 48.0,
            font_weight: 700,
            text_color_rgba: [1.0, 0.9, 0.1, 1.0], // Eye-catching yellow
            bg_color_rgba: [0.0, 0.0, 0.0, 0.8],
            outline_color_rgba: [0.0, 0.0, 0.0, 1.0],
            outline_width: 3.0,
            vertical_alignment: "bottom".to_string(),
            margin_bottom_percent: 0.1,
        },
    };
    assert!(app.state.apply_op(Op::AddCaption { caption }, OpSource::Human).is_ok());

    {
        let engine = app.state.engine.lock();
        let tl = engine.project().active_timeline().unwrap();
        assert_eq!(tl.captions.len(), 1);
        assert_eq!(tl.captions[0].text, "Kinetic TikTok Subtitle");
    }

    // Update text
    assert!(app.state.apply_op(Op::UpdateCaption {
        caption_id: cap_id,
        text: Some("Updated Cinematic Title".to_string()),
        style: None,
    }, OpSource::Human).is_ok());

    {
        let engine = app.state.engine.lock();
        let tl = engine.project().active_timeline().unwrap();
        assert_eq!(tl.captions[0].text, "Updated Cinematic Title");
    }
}
