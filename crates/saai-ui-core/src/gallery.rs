//! Labelled fixture data for the VUI-05 composite gallery plus VUI-07
//! `EventRow`/`SpaceRow`/`WifiRow`/`BluetoothRow`/`TrustedClientRow`.
//! No live telemetry. `DecisionOverlay` and `TrustedClientRow` are
//! privileged (ADR-185); they are review fixtures, not a public subset.

use crate::{
    AgentSummary, BluetoothRow, ContextHeader, DecisionOverlay, EventRow, IntentSummary,
    ObjectSummary, SpaceRow, StatusIndicator, SurfacePattern, TaskSummary, TrustedClientRow,
    UniversalState, WifiRow,
};

pub struct CompositeGalleryFixtures {
    pub title: &'static str,
    pub header: ContextHeader,
    pub object: ObjectSummary,
    pub task: TaskSummary,
    pub intent: IntentSummary,
    pub agent_unassigned: AgentSummary,
    pub agent_assigned: AgentSummary,
    pub decision: DecisionOverlay,
    pub event_decision: EventRow,
    pub event_notice: EventRow,
    pub space_selected: SpaceRow,
    pub space_other: SpaceRow,
    pub wifi_connected: WifiRow,
    pub wifi_other: WifiRow,
    pub wifi_empty: WifiRow,
    pub bluetooth_paired: BluetoothRow,
    pub bluetooth_other: BluetoothRow,
    pub bluetooth_empty: BluetoothRow,
    pub bluetooth_loading: BluetoothRow,
    pub trusted_named: TrustedClientRow,
    pub trusted_empty: TrustedClientRow,
    pub states: [StatusIndicator; 9],
    pub patterns: [SurfacePattern; 5],
}

pub fn composite_gallery_fixtures() -> CompositeGalleryFixtures {
    CompositeGalleryFixtures {
        title: "Композиты · VUI-05",
        header: ContextHeader::new("Работа")
            .with_section_title("Сейчас")
            .with_lifecycle(StatusIndicator::new(UniversalState::Offline, "Нет связи")),
        object: ObjectSummary::new("Отчёт", "saaios.document · версия 3"),
        task: TaskSummary::new("Экспорт PDF", UniversalState::Running).with_reason("Выполняется"),
        intent: IntentSummary::new("Пустое намерение"),
        agent_unassigned: AgentSummary::unassigned(),
        agent_assigned: AgentSummary::from_execution("Экспорт PDF", UniversalState::Running),
        decision: DecisionOverlay::new("Подтвердите: удалить файл")
            .with_actor("Система")
            .with_action("files.delete")
            .with_scope("работа"),
        event_decision: EventRow::decision("Подтвердите удаление"),
        event_notice: EventRow::notice("Notice", "body"),
        space_selected: SpaceRow::open("Дом", "Объектов: 3", "select_space:home", true),
        space_other: SpaceRow::open("Работа", "Объектов: 1", "select_space:work", false),
        wifi_connected: WifiRow::open("Wallbox", "защищена · -42 dBm", true),
        wifi_other: WifiRow::open("Guest", "открыта · -70 dBm", false),
        wifi_empty: WifiRow::empty(),
        bluetooth_paired: BluetoothRow::open("Pixel Buds", "BLE", true),
        bluetooth_other: BluetoothRow::open("Speaker", "CLASSIC", false),
        bluetooth_empty: BluetoothRow::empty(),
        bluetooth_loading: BluetoothRow::loading(),
        trusted_named: TrustedClientRow::open("home-mike", "SHA256:abcdabcdabcdabcdabcdabcd…"),
        trusted_empty: TrustedClientRow::empty(),
        states: UniversalState::ALL
            .map(|state| StatusIndicator::new(state, state_fixture_label(state))),
        patterns: [
            SurfacePattern::empty("Ничего срочного"),
            SurfacePattern::loading("Сканирование…"),
            SurfacePattern::offline("Нет связи"),
            SurfacePattern::blocked("Нет адаптера"),
            SurfacePattern::failed("Ошибка сопряжения: timeout"),
        ],
    }
}

pub fn public_gallery_type_names() -> &'static [&'static str] {
    &[
        "ContextHeader",
        "ObjectSummary",
        "TaskSummary",
        "IntentSummary",
        "AgentSummary",
        "EventRow",
        "SpaceRow",
        "WifiRow",
        "BluetoothRow",
        "StatusIndicator",
        "SurfacePattern",
    ]
}

pub fn privileged_gallery_type_names() -> &'static [&'static str] {
    &["DecisionOverlay", "TrustedClientRow"]
}

pub fn state_fixture_label(state: UniversalState) -> &'static str {
    match state {
        UniversalState::Idle => "Спокойно",
        UniversalState::Active => "Активно",
        UniversalState::Running => "Выполняется",
        UniversalState::Waiting => "Ожидает",
        UniversalState::Blocked => "Заблокировано",
        UniversalState::Attention => "Внимание",
        UniversalState::Failed => "Ошибка",
        UniversalState::Complete => "Готово",
        UniversalState::Offline => "Нет связи",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentAssignment;

    #[test]
    fn composite_gallery_fixtures_cover_every_universal_state() {
        let fixtures = composite_gallery_fixtures();
        let states: Vec<_> = fixtures
            .states
            .iter()
            .map(|indicator| indicator.state)
            .collect();
        assert_eq!(states.as_slice(), &UniversalState::ALL);
        for indicator in &fixtures.states {
            assert_eq!(indicator.label, state_fixture_label(indicator.state));
        }
        assert_eq!(
            fixtures
                .patterns
                .iter()
                .map(|pattern| pattern.state)
                .collect::<Vec<_>>(),
            vec![
                UniversalState::Idle,
                UniversalState::Waiting,
                UniversalState::Offline,
                UniversalState::Blocked,
                UniversalState::Failed,
            ]
        );
    }

    #[test]
    fn composite_gallery_fixtures_are_labelled_without_fake_workers() {
        let fixtures = composite_gallery_fixtures();
        assert_eq!(fixtures.title, "Композиты · VUI-05");
        assert_eq!(fixtures.header.context_name, "Работа");
        assert_eq!(fixtures.object.title, "Отчёт");
        assert_eq!(fixtures.task.state, UniversalState::Running);
        assert!(fixtures.intent.task.is_none());
        assert_eq!(
            fixtures.intent.missing_task_text().map(|text| text.content),
            Some("Нет задачи".into())
        );
        assert_eq!(
            fixtures.agent_unassigned.assignment,
            AgentAssignment::Unassigned
        );
        assert_eq!(fixtures.agent_unassigned.detail_line(), "Нет исполнения");
        assert_eq!(
            fixtures.agent_assigned.detail_line(),
            "Исполнение: Экспорт PDF"
        );
        assert_eq!(fixtures.decision.accept.label, "Подтвердить");
        assert_eq!(fixtures.decision.decline.label, "Отклонить");
        assert_eq!(fixtures.event_decision.row.primary, "Подтвердите удаление");
        assert_eq!(fixtures.event_notice.row.value.as_deref(), Some("body"));
        assert!(fixtures.space_selected.selected);
        assert!(!fixtures.space_other.selected);
        assert!(fixtures.wifi_connected.connected);
        assert!(!fixtures.wifi_other.connected);
        assert_eq!(fixtures.wifi_empty.row.primary, "Нет сетей");
        assert!(fixtures.bluetooth_paired.paired);
        assert!(!fixtures.bluetooth_other.paired);
        assert_eq!(fixtures.bluetooth_empty.row.primary, "Нет устройств");
        assert_eq!(fixtures.bluetooth_loading.row.primary, "Сканирование…");
        assert_eq!(fixtures.trusted_named.row.primary, "home-mike");
        assert_eq!(fixtures.trusted_empty.row.primary, "Нет клиентов");
        assert_eq!(fixtures.patterns[0].message, "Ничего срочного");
        assert!(!fixtures.patterns[0].paints_mark());
        assert!(fixtures.patterns[1].paints_mark());
        assert!(fixtures.patterns[4].paints_mark());
        assert_eq!(fixtures.patterns[1].message, "Сканирование…");
        assert_eq!(fixtures.patterns[2].message, "Нет связи");
        assert_eq!(fixtures.patterns[3].message, "Нет адаптера");
        assert_eq!(fixtures.patterns[4].message, "Ошибка сопряжения: timeout");
        let blob = format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            fixtures.title,
            fixtures.header.context_name,
            fixtures.object.title,
            fixtures.task.title,
            fixtures.intent.title,
            fixtures.agent_assigned.detail_line(),
            fixtures.event_decision.row.primary,
            fixtures.event_notice.row.primary,
            fixtures.space_selected.row.primary,
            fixtures.space_other.row.primary,
            fixtures.wifi_connected.row.primary,
            fixtures.wifi_empty.row.primary,
            fixtures.bluetooth_paired.row.primary,
            fixtures.trusted_named.row.primary
        );
        assert!(!blob.contains("Воркеры"));
        assert!(!blob.contains("ResearchAgent"));
        assert!(!blob.contains("% CPU"));
    }

    #[test]
    fn gallery_type_names_split_public_from_privileged() {
        for name in public_gallery_type_names() {
            assert!(
                !privileged_gallery_type_names().contains(name),
                "{name} listed as both public and privileged"
            );
        }
        assert_eq!(
            privileged_gallery_type_names(),
            &["DecisionOverlay", "TrustedClientRow"]
        );
        assert!(public_gallery_type_names().contains(&"ContextHeader"));
        assert!(public_gallery_type_names().contains(&"SurfacePattern"));
        assert!(!public_gallery_type_names().contains(&"OrbHost"));
    }
}
