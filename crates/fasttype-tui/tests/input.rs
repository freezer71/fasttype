use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_tui::input::{Input, Key, Phase, map_event, map_key};

fn k(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)
}

#[test]
fn maps_typing_keys() {
    assert_eq!(
        map_key(&k(KeyCode::Char('a'), KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Char('a')
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('A'), KeyModifiers::SHIFT))
            .unwrap()
            .0,
        Key::Char('A')
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('é'), KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Char('é')
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Backspace
    );
}

#[test]
fn maps_shortcuts() {
    assert_eq!(
        map_key(&k(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::Quit
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::ALT))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('w'), KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Enter, KeyModifiers::SHIFT)).unwrap().0,
        Key::ShiftEnter
    );
    assert_eq!(
        map_key(&k(KeyCode::Tab, KeyModifiers::NONE)).unwrap().0,
        Key::Tab
    );
    assert_eq!(
        map_key(&k(KeyCode::Esc, KeyModifiers::NONE)).unwrap().0,
        Key::Esc
    );
    assert!(map_key(&k(KeyCode::Char('x'), KeyModifiers::CONTROL)).is_none());
    assert!(map_key(&k(KeyCode::F(1), KeyModifiers::NONE)).is_none());
}

#[test]
fn press_and_release_share_a_code() {
    let (_, down) = map_key(&k(KeyCode::Char('A'), KeyModifiers::SHIFT)).unwrap();
    let (_, up) = map_key(&KeyEvent::new_with_kind(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ))
    .unwrap();
    assert_eq!(down, up);
}

#[test]
fn events_carry_phase_and_time() {
    let ev = Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    assert_eq!(
        map_event(ev, 12.5),
        Some(Input::Key {
            key: Key::Char('a'),
            phase: Phase::Release,
            code: 'a' as u32,
            at: 12.5
        })
    );
    assert_eq!(map_event(Event::Resize(80, 24), 0.0), Some(Input::Resize));
    assert_eq!(map_event(Event::FocusGained, 0.0), None);
}
