//! Labelled fixture data for the VUI-05 composite gallery.
//! No live telemetry. `EventRow` is still deferred.

use crate::{
    AgentSummary, ContextHeader, DecisionOverlay, IntentSummary, ObjectSummary, StatusIndicator,
    TaskSummary, UniversalState,
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
    pub states: [StatusIndicator; 9],
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
        states: UniversalState::ALL
            .map(|state| StatusIndicator::new(state, state_fixture_label(state))),
    }
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
        let blob = format!(
            "{} {} {} {} {} {}",
            fixtures.title,
            fixtures.header.context_name,
            fixtures.object.title,
            fixtures.task.title,
            fixtures.intent.title,
            fixtures.agent_assigned.detail_line()
        );
        assert!(!blob.contains("Воркеры"));
        assert!(!blob.contains("ResearchAgent"));
        assert!(!blob.contains("% CPU"));
    }
}
