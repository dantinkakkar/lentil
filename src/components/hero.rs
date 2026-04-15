use std::process::Command;

use dioxus::{desktop::use_window, prelude::*};

use crate::config::LentilConfig;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailStep {
    Project,
    Due,
    Priority,
    Tags,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureState {
    Title,
    Details(DetailStep),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SuggestionKind {
    Project,
    Tag,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct MetadataDraft {
    project_input: String,
    due_input: String,
    due_normalized: Option<String>,
    due_implicit_default: bool,
    priority: Option<char>,
    tags_input: String,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct SuggestionCache {
    projects: Vec<String>,
    tags: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LocalDate {
    day: u32,
    month: u32,
    year: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CalendarState {
    day: u32,
    month: u32,
    year: i32,
}

#[derive(Clone, PartialEq, Eq)]
struct CalendarCell {
    label: String,
    display: Option<String>,
    is_selected: bool,
    is_today: bool,
}

impl CaptureState {
    fn input_id(self) -> &'static str {
        match self {
            CaptureState::Title => "capture-title-input",
            CaptureState::Details(DetailStep::Project) => "capture-project-input",
            CaptureState::Details(DetailStep::Due) => "capture-due-input",
            CaptureState::Details(DetailStep::Priority) => "capture-priority-input",
            CaptureState::Details(DetailStep::Tags) => "capture-tags-input",
        }
    }

    fn section_id(self) -> &'static str {
        match self {
            CaptureState::Title => "capture-title-section",
            CaptureState::Details(DetailStep::Project) => "capture-project-section",
            CaptureState::Details(DetailStep::Due) => "capture-due-section",
            CaptureState::Details(DetailStep::Priority) => "capture-priority-section",
            CaptureState::Details(DetailStep::Tags) => "capture-tags-section",
        }
    }

    fn details_visible(self) -> bool {
        matches!(self, CaptureState::Details(_))
    }

    fn enter_details(self) -> Self {
        match self {
            CaptureState::Title => CaptureState::Details(DetailStep::Project),
            state => state,
        }
    }

    fn retreat(self) -> Self {
        match self {
            CaptureState::Title => CaptureState::Title,
            CaptureState::Details(DetailStep::Project) => CaptureState::Title,
            CaptureState::Details(DetailStep::Due) => CaptureState::Details(DetailStep::Project),
            CaptureState::Details(DetailStep::Priority) => CaptureState::Details(DetailStep::Due),
            CaptureState::Details(DetailStep::Tags) => CaptureState::Details(DetailStep::Priority),
        }
    }

    fn advance(self) -> Self {
        match self {
            CaptureState::Title => CaptureState::Details(DetailStep::Project),
            CaptureState::Details(DetailStep::Project) => CaptureState::Details(DetailStep::Due),
            CaptureState::Details(DetailStep::Due) => CaptureState::Details(DetailStep::Priority),
            CaptureState::Details(DetailStep::Priority) => CaptureState::Details(DetailStep::Tags),
            CaptureState::Details(DetailStep::Tags) => CaptureState::Details(DetailStep::Tags),
        }
    }
}

fn is_eoi_signal(event: &KeyboardEvent) -> bool {
    event.key() == Key::Character("d".into()) && event.modifiers().ctrl()
}

fn current_local_date() -> LocalDate {
    let output = Command::new("date").arg("+%d %m %Y").output();

    let Ok(output) = output else {
        return LocalDate {
            day: 1,
            month: 1,
            year: 1970,
        };
    };

    let value = String::from_utf8_lossy(&output.stdout);
    let mut parts = value.split_whitespace();

    let day = parts.next().and_then(|value| value.parse::<u32>().ok());
    let month = parts.next().and_then(|value| value.parse::<u32>().ok());
    let year = parts.next().and_then(|value| value.parse::<i32>().ok());

    match (day, month, year) {
        (Some(day), Some(month), Some(year)) => LocalDate { day, month, year },
        _ => LocalDate {
            day: 1,
            month: 1,
            year: 1970,
        },
    }
}

fn parse_display_date(display: &str) -> Option<CalendarState> {
    let mut parts = display.split('/');
    let day = parts.next()?.parse::<u32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let year = parts.next()?.parse::<i32>().ok()?;
    Some(CalendarState { day, month, year })
}

fn display_date_to_taskwarrior(display: &str) -> Option<String> {
    let state = parse_display_date(display)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        state.year, state.month, state.day
    ))
}

fn normalize_due_input(raw: &str, today: LocalDate) -> Result<Option<String>, String> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    let segments: Vec<&str> = trimmed.split('/').collect();

    let (day, month, year) = match segments.as_slice() {
        [day] => {
            let parsed = parse_number(day, "day")?;
            if parsed <= 12 {
                (today.day, parsed, today.year)
            } else {
                (parsed, today.month, today.year)
            }
        }
        ["", month] => (today.day, parse_number(month, "month")?, today.year),
        [day, month] => (
            parse_number(day, "day")?,
            parse_number(month, "month")?,
            today.year,
        ),
        ["", month, year] => (today.day, parse_number(month, "month")?, parse_year(year)?),
        [day, month, year] => (
            parse_number(day, "day")?,
            parse_number(month, "month")?,
            parse_year(year)?,
        ),
        _ => return Err("Use dd, dd/mm, /mm, dd/mm/YYYY, or /mm/YYYY".into()),
    };

    validate_date(day, month, year)
}

fn parse_number(value: &str, label: &str) -> Result<u32, String> {
    value.parse::<u32>().map_err(|_| format!("Invalid {label}"))
}

fn parse_year(value: &str) -> Result<i32, String> {
    value.parse::<i32>().map_err(|_| "Invalid year".to_string())
}

fn validate_date(day: u32, month: u32, year: i32) -> Result<Option<String>, String> {
    let input = format!("{year:04}-{month:02}-{day:02}");
    let output = Command::new("date")
        .args(["-d", &input, "+%d/%m/%Y"])
        .output()
        .map_err(|error| format!("Unable to validate date: {error}"))?;

    if output.status.success() {
        let normalized = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(normalized))
    } else {
        Err("That date does not exist".into())
    }
}

fn shift_display_date(display: &str, delta_days: i32) -> Option<String> {
    let state = parse_display_date(display)?;
    let input = format!("{:04}-{:02}-{:02}", state.year, state.month, state.day);
    let relative = if delta_days >= 0 {
        format!("{input} +{delta_days} day")
    } else {
        format!("{input} {delta_days} day")
    };

    let output = Command::new("date")
        .args(["-d", &relative, "+%d/%m/%Y"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn resolve_due_arrow_target(
    draft: &MetadataDraft,
    today: LocalDate,
    delta_days: i32,
) -> Option<String> {
    if draft.due_input.trim().is_empty() && draft.due_implicit_default {
        Some(today_display(today))
    } else {
        let current = draft
            .due_normalized
            .clone()
            .unwrap_or_else(|| today_display(today));
        shift_display_date(&current, delta_days)
    }
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Month",
    }
}

fn month_grid(calendar: CalendarState, today: LocalDate) -> Vec<CalendarCell> {
    let first_weekday = month_first_weekday(calendar.month, calendar.year).unwrap_or(1);
    let last_day = month_last_day(calendar.month, calendar.year).unwrap_or(30);
    let mut cells = Vec::new();

    for _ in 1..first_weekday {
        cells.push(CalendarCell {
            label: String::new(),
            display: None,
            is_selected: false,
            is_today: false,
        });
    }

    for day in 1..=last_day {
        let display = format!("{day:02}/{:02}/{:04}", calendar.month, calendar.year);
        cells.push(CalendarCell {
            label: day.to_string(),
            display: Some(display),
            is_selected: day == calendar.day,
            is_today: day == today.day
                && calendar.month == today.month
                && calendar.year == today.year,
        });
    }

    cells
}

fn month_first_weekday(month: u32, year: i32) -> Option<u32> {
    let input = format!("{year:04}-{month:02}-01");
    let output = Command::new("date")
        .args(["-d", &input, "+%u"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .ok()
    } else {
        None
    }
}

fn month_last_day(month: u32, year: i32) -> Option<u32> {
    let input = format!("{year:04}-{month:02}-01 +1 month -1 day");
    let output = Command::new("date")
        .args(["-d", &input, "+%d"])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .ok()
    } else {
        None
    }
}

fn load_suggestions() -> SuggestionCache {
    SuggestionCache {
        projects: load_task_list("_projects"),
        tags: load_task_list("_tags"),
    }
}

fn load_task_list(task_arg: &str) -> Vec<String> {
    let output = Command::new("task").arg(task_arg).output();
    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_tags(input: &str) -> Vec<String> {
    let mut tags = Vec::new();

    for part in input
        .split(|character: char| character.is_whitespace() || character == ',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let tag = part.trim_start_matches('#').to_ascii_lowercase();
        if !tag.is_empty() {
            tags.push(tag);
        }
    }

    tags.sort();
    tags.dedup();
    tags
}

fn suggestion_score(candidate: &str, query: &str, title: &str) -> i32 {
    let candidate_lower = candidate.to_ascii_lowercase();
    let query_lower = query.trim().to_ascii_lowercase();
    let title_lower = title.to_ascii_lowercase();
    let mut score = 0;

    if query_lower.is_empty() {
        score += 5;
    } else if candidate_lower == query_lower {
        score += 100;
    } else if candidate_lower.starts_with(&query_lower) {
        score += 70;
    } else if candidate_lower.contains(&query_lower) {
        score += 35;
    }

    if title_lower.contains(&candidate_lower) {
        score += 25;
    }

    score
}

fn ranked_suggestions(
    items: &[String],
    query: &str,
    title: &str,
    kind: SuggestionKind,
    config: &LentilConfig,
    due_is_empty: bool,
) -> Vec<String> {
    let mut ranked: Vec<(i32, String)> = items
        .iter()
        .map(|item| (suggestion_score(item, query, title), item.clone()))
        .filter(|(score, item)| *score > 0 || query.is_empty() || item.contains(query))
        .collect();

    if kind == SuggestionKind::Tag && config.suggest_idea_without_due && due_is_empty {
        ranked.push((45, "idea".to_string()));
    }

    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.len().cmp(&right.1.len()))
            .then_with(|| left.1.cmp(&right.1))
    });

    let mut unique = Vec::new();
    for (_, item) in ranked {
        if !unique.contains(&item) {
            unique.push(item);
        }
    }

    unique.truncate(6);
    unique
}

fn apply_selected_suggestion(
    selected_index: usize,
    suggestions: &[String],
    current_value: &str,
    kind: SuggestionKind,
) -> String {
    let picked = suggestions
        .get(selected_index)
        .cloned()
        .filter(|value| !value.is_empty());

    match kind {
        SuggestionKind::Project => picked.unwrap_or_else(|| current_value.trim().to_string()),
        SuggestionKind::Tag => merge_tag_input(current_value, picked.as_deref()),
    }
}

fn selected_suggestion(selected_index: usize, suggestions: &[String]) -> Option<String> {
    suggestions
        .get(selected_index)
        .cloned()
        .filter(|value| !value.is_empty())
}

fn next_priority(current: Option<char>) -> char {
    match current {
        Some('H') => 'M',
        Some('M') => 'L',
        _ => 'H',
    }
}

fn previous_priority(current: Option<char>) -> char {
    match current {
        Some('L') => 'M',
        Some('M') => 'H',
        _ => 'L',
    }
}

fn merge_tag_input(current: &str, selected: Option<&str>) -> String {
    let Some(selected) = selected else {
        return current.trim().to_string();
    };

    let mut tags = normalize_tags(current);
    if !tags.iter().any(|tag| tag == selected) {
        tags.push(selected.to_string());
    }
    tags.sort();
    tags.dedup();

    tags.into_iter()
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_task_command(title: &str, draft: &MetadataDraft) -> Command {
    let mut command = Command::new("task");
    command.arg("add").arg(title.trim());

    if !draft.project_input.trim().is_empty() {
        command.arg(format!("project:{}", draft.project_input.trim()));
    }

    if let Some(due) = draft
        .due_normalized
        .as_deref()
        .and_then(display_date_to_taskwarrior)
    {
        command.arg(format!("due:{due}"));
    }

    if let Some(priority) = draft.priority {
        command.arg(format!("priority:{priority}"));
    }

    for tag in normalize_tags(&draft.tags_input) {
        command.arg(format!("+{tag}"));
    }

    command
}

fn run_taskwarrior(title: &str, draft: &MetadataDraft) -> Result<String, String> {
    let output = build_task_command(title, draft)
        .output()
        .map_err(|error| format!("Unable to start Taskwarrior: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        if stdout.is_empty() {
            Ok("Task added".into())
        } else {
            Ok(stdout)
        }
    } else if stderr.is_empty() {
        Err("Taskwarrior rejected the task".into())
    } else {
        Err(stderr)
    }
}

fn due_preview_text(draft: &MetadataDraft) -> &'static str {
    if draft.due_input.trim().is_empty() && draft.due_implicit_default {
        "Defaulting to today. Type dd, dd/mm, /mm, dd/mm/YYYY, or /mm/YYYY."
    } else if draft.due_input.trim().is_empty() {
        "Optional. Type dd, dd/mm, /mm, dd/mm/YYYY, or /mm/YYYY."
    } else if draft.due_normalized.is_some() {
        "Resolved date"
    } else {
        "Complete the date to continue"
    }
}

fn today_display(today: LocalDate) -> String {
    format!("{:02}/{:02}/{:04}", today.day, today.month, today.year)
}

#[component]
pub fn Hero() -> Element {
    let window = use_window();
    let today = use_signal(current_local_date);
    let config = use_signal(LentilConfig::load);
    let suggestions = use_signal(load_suggestions);
    let mut step = use_signal(|| CaptureState::Title);
    let mut title_value = use_signal(String::new);
    let mut draft = use_signal(MetadataDraft::default);
    let mut project_selection = use_signal(|| 0usize);
    let mut tag_selection = use_signal(|| 0usize);
    let mut toast_msg = use_signal(String::new);
    let mut toast_is_error = use_signal(|| false);

    let project_suggestions = ranked_suggestions(
        &suggestions.read().projects,
        &draft.read().project_input,
        &title_value.read(),
        SuggestionKind::Project,
        &config.read(),
        draft.read().due_normalized.is_none(),
    );
    let tag_suggestions = ranked_suggestions(
        &suggestions.read().tags,
        &draft.read().tags_input,
        &title_value.read(),
        SuggestionKind::Tag,
        &config.read(),
        draft.read().due_normalized.is_none(),
    );

    let calendar_value = draft.read().due_normalized.clone().unwrap_or_else(|| {
        format!(
            "{:02}/{:02}/{:04}",
            today.read().day,
            today.read().month,
            today.read().year
        )
    });
    let calendar_state = parse_display_date(&calendar_value).unwrap_or(CalendarState {
        day: today.read().day,
        month: today.read().month,
        year: today.read().year,
    });
    let calendar_cells = month_grid(calendar_state, *today.read());
    let project_suggestions_for_forward = project_suggestions.clone();
    let project_suggestions_for_keys = project_suggestions.clone();
    let project_suggestions_for_list = project_suggestions.clone();
    let tag_suggestions_for_keys = tag_suggestions.clone();
    let tag_suggestions_for_list = tag_suggestions.clone();

    let mut reset_capture = move || {
        title_value.set(String::new());
        draft.set(MetadataDraft::default());
        project_selection.set(0);
        tag_selection.set(0);
        step.set(CaptureState::Title);
    };

    let mut show_error = move |message: String| {
        toast_msg.set(message);
        toast_is_error.set(true);
    };

    let mut show_message = move |message: String| {
        toast_msg.set(message);
        toast_is_error.set(false);
    };

    let mut sync_due_input = move |value: String| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            let fallback = today_display(today());
            draft.with_mut(|draft| {
                draft.due_input.clear();
                if draft.due_implicit_default {
                    draft.due_normalized = Some(fallback.clone());
                } else {
                    draft.due_normalized = None;
                }
            });
            toast_msg.set(String::new());
            toast_is_error.set(false);
            return;
        }

        let normalized = normalize_due_input(&value, today());
        draft.with_mut(|draft| {
            draft.due_input = value;
            draft.due_normalized = normalized.clone().ok().flatten();
            draft.due_implicit_default = false;
        });

        match normalized {
            Ok(_) => {
                toast_msg.set(String::new());
                toast_is_error.set(false);
            }
            Err(message) => {
                toast_msg.set(message);
                toast_is_error.set(true);
            }
        }
    };

    let submit_window = window.clone();
    let mut submit_capture = move || {
        if title_value().trim().is_empty() {
            show_error("Task name cannot be empty".to_string());
            step.set(CaptureState::Title);
            return;
        }

        if config().require_priority_with_due
            && draft.read().due_normalized.is_some()
            && draft.read().priority.is_none()
        {
            show_error("Priority is required when a due date is set".to_string());
            step.set(CaptureState::Details(DetailStep::Priority));
            return;
        }

        match run_taskwarrior(&title_value(), &draft()) {
            Ok(message) => {
                show_message(message);

                if config().clear_and_stay_open {
                    reset_capture();
                } else {
                    submit_window.close();
                }
            }
            Err(message) => show_error(message),
        }
    };

    let mut handle_forward = move || match step() {
        CaptureState::Title => {
            if title_value().trim().is_empty() {
                show_error("Task name cannot be empty".to_string());
            } else {
                toast_msg.set(String::new());
                toast_is_error.set(false);
                step.set(step().enter_details());
            }
        }
        CaptureState::Details(DetailStep::Project) => {
            let updated = apply_selected_suggestion(
                project_selection(),
                &project_suggestions_for_forward,
                &draft.read().project_input,
                SuggestionKind::Project,
            );
            let today_display = today_display(today());
            draft.with_mut(|draft| {
                draft.project_input = updated;
                if draft.due_input.trim().is_empty() && draft.due_normalized.is_none() {
                    draft.due_input.clear();
                    draft.due_normalized = Some(today_display);
                    draft.due_implicit_default = true;
                }
            });
            step.set(step().advance());
        }
        CaptureState::Details(DetailStep::Due) => {
            let due_input = draft.read().due_input.clone();
            match normalize_due_input(&due_input, today()) {
                Ok(normalized) => {
                    draft.with_mut(|draft| draft.due_normalized = normalized);
                    step.set(step().advance());
                }
                Err(message) => show_error(message),
            }
        }
        CaptureState::Details(DetailStep::Priority) => {
            if config().require_priority_with_due
                && draft.read().due_normalized.is_some()
                && draft.read().priority.is_none()
            {
                show_error("Priority is required when a due date is set".to_string());
            } else {
                step.set(step().advance());
            }
        }
        CaptureState::Details(DetailStep::Tags) => submit_capture(),
    };

    use_effect(move || {
        let active_input = step().input_id();
        let active_section = step().section_id();
        let _title_length = title_value();
        document::eval(&format!(
            r#"
            const resizeTitleInput = () => {{
                const titleInput = document.getElementById("capture-title-input");
                if (!titleInput) {{
                    return;
                }}

                titleInput.style.height = "0px";
                const nextHeight = Math.min(Math.max(titleInput.scrollHeight + 12, 52), 360);
                titleInput.style.height = `${{nextHeight}}px`;
            }};

            const input = document.getElementById({active_input:?});
            if (input) {{
                input.focus();
                if (typeof input.setSelectionRange === "function") {{
                    const len = input.value?.length ?? 0;
                    input.setSelectionRange(len, len);
                }}
            }}

            const section = document.getElementById({active_section:?});
            if (section) {{
                section.scrollIntoView({{ block: "nearest", behavior: "instant" }});
            }}

            resizeTitleInput();
            window.requestAnimationFrame(resizeTitleInput);
            "#
        ));
    });

    use_effect(move || {
        document::eval(
            r#"
            if (!window.__lentilTabSuppressed) {
                const suppressTab = (event) => {
                    if (event.key === "Tab" || event.code === "Tab") {
                        event.preventDefault();
                        event.stopPropagation();
                        if (typeof event.stopImmediatePropagation === "function") {
                            event.stopImmediatePropagation();
                        }
                    }
                };

                window.addEventListener("keydown", suppressTab, true);
                window.addEventListener("keyup", suppressTab, true);
                document.addEventListener("keydown", suppressTab, true);
                document.addEventListener("keyup", suppressTab, true);
                window.__lentilTabSuppressed = true;
            }
            "#,
        );
    });

    rsx! {
        div {
            class: "capture-shell",
            onkeydown: move |event| {
                if event.key() == Key::Tab {
                    event.prevent_default();
                } else if is_eoi_signal(&event) {
                    event.prevent_default();
                    handle_forward();
                } else if event.key() == Key::Escape {
                    event.prevent_default();

                    if step() == CaptureState::Title {
                        if title_value().is_empty() {
                            window.close();
                        } else {
                            title_value.set(String::new());
                            toast_msg.set(String::new());
                            toast_is_error.set(false);
                        }
                    } else {
                        step.set(step().retreat());
                    }
                }
            },
            div {
                class: "capture-panel",
                div {
                    class: "capture-panel-body",
                    div {
                        id: "capture-title-section",
                        class: "capture-title-lockup",
                        p { class: "capture-label", "Capture" }
                        textarea {
                            id: "capture-title-input",
                            class: "capture-input capture-input-title",
                            value: "{title_value}",
                            autofocus: true,
                            placeholder: "What needs doing?",
                            rows: "1",
                            onfocus: move |_| step.set(CaptureState::Title),
                            oninput: move |event| title_value.set(event.value()),
                        }
                    }

                    if step().details_visible() {
                        div { class: "capture-divider" }
                        div {
                            class: "capture-guided-flow",
                            if step() == CaptureState::Details(DetailStep::Project) {
                                div {
                                    id: "capture-project-section",
                                    class: "capture-section capture-section-active",
                                    p { class: "capture-label", "Project" }
                                    input {
                                        id: "capture-project-input",
                                        class: "capture-input capture-input-detail",
                                        value: "{draft.read().project_input}",
                                        placeholder: "Type or pick a project",
                                        autofocus: true,
                                        onfocus: move |_| step.set(CaptureState::Details(DetailStep::Project)),
                                        oninput: move |event| {
                                            project_selection.set(0);
                                            draft.with_mut(|draft| draft.project_input = event.value());
                                        },
                                        onkeydown: move |event| {
                                            if step() != CaptureState::Details(DetailStep::Project) {
                                                return;
                                            }

                                            match event.key() {
                                                Key::ArrowRight | Key::ArrowDown => {
                                                    event.prevent_default();
                                                    if !project_suggestions_for_keys.is_empty() {
                                                        let next = (project_selection() + 1)
                                                            .min(project_suggestions_for_keys.len().saturating_sub(1));
                                                        project_selection.set(next);
                                                        if let Some(selected) = selected_suggestion(next, &project_suggestions_for_keys) {
                                                            draft.with_mut(|draft| draft.project_input = selected);
                                                        }
                                                    }
                                                }
                                                Key::ArrowLeft | Key::ArrowUp => {
                                                    event.prevent_default();
                                                    let next = project_selection().saturating_sub(1);
                                                    project_selection.set(next);
                                                    if let Some(selected) = selected_suggestion(next, &project_suggestions_for_keys) {
                                                        draft.with_mut(|draft| draft.project_input = selected);
                                                    }
                                                }
                                                Key::Enter => {
                                                    event.prevent_default();
                                                    let updated = apply_selected_suggestion(
                                                        project_selection(),
                                                        &project_suggestions_for_keys,
                                                        &draft.read().project_input,
                                                        SuggestionKind::Project,
                                                    );
                                                    draft.with_mut(|draft| draft.project_input = updated);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    if !project_suggestions.is_empty() {
                                        p { class: "capture-supporting-copy", "Arrow keys select. Enter accepts." }
                                        div { class: "capture-suggestions",
                                            for (index, item) in project_suggestions_for_list.clone().into_iter().enumerate() {
                                                button {
                                                    class: if index == project_selection() { "capture-chip capture-chip-selected" } else { "capture-chip" },
                                                    tabindex: "-1",
                                                    onclick: move |_| {
                                                        project_selection.set(index);
                                                        draft.with_mut(|draft| draft.project_input = item.clone());
                                                    },
                                                    "{item}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if step() == CaptureState::Details(DetailStep::Due) {
                                div {
                                    id: "capture-due-section",
                                    class: "capture-section capture-section-active",
                                    p { class: "capture-label", "Due" }
                                    input {
                                        id: "capture-due-input",
                                        class: "capture-input capture-input-detail",
                                        value: "{draft.read().due_input}",
                                        placeholder: "Optional due date",
                                        autofocus: true,
                                        onfocus: move |_| step.set(CaptureState::Details(DetailStep::Due)),
                                        oninput: move |event| sync_due_input(event.value()),
                                        onkeydown: move |event| {
                                            if step() != CaptureState::Details(DetailStep::Due) {
                                                return;
                                            }

                                            match event.key() {
                                                Key::ArrowLeft => {
                                                    event.prevent_default();
                                                    if let Some(updated) = resolve_due_arrow_target(&draft.read(), today(), -1) {
                                                        sync_due_input(updated);
                                                    }
                                                }
                                                Key::ArrowRight => {
                                                    event.prevent_default();
                                                    if let Some(updated) = resolve_due_arrow_target(&draft.read(), today(), 1) {
                                                        sync_due_input(updated);
                                                    }
                                                }
                                                Key::ArrowUp => {
                                                    event.prevent_default();
                                                    if let Some(updated) = resolve_due_arrow_target(&draft.read(), today(), -7) {
                                                        sync_due_input(updated);
                                                    }
                                                }
                                                Key::ArrowDown => {
                                                    event.prevent_default();
                                                    if let Some(updated) = resolve_due_arrow_target(&draft.read(), today(), 7) {
                                                        sync_due_input(updated);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    p { class: "capture-supporting-copy", "{due_preview_text(&draft.read())}" }
                                    p {
                                        class: "capture-preview",
                                        if let Some(due) = draft.read().due_normalized.as_deref() {
                                            "{due}"
                                        } else {
                                            "No due date"
                                        }
                                    }
                                    div {
                                        class: "capture-calendar",
                                        div { class: "capture-calendar-head",
                                            p { class: "capture-calendar-title", "{month_name(calendar_state.month)} {calendar_state.year}" }
                                            p { class: "capture-calendar-meta", "Arrow keys move the selection" }
                                        }
                                        div { class: "capture-calendar-weekdays",
                                            span { "Mo" }
                                            span { "Tu" }
                                            span { "We" }
                                            span { "Th" }
                                            span { "Fr" }
                                            span { "Sa" }
                                            span { "Su" }
                                        }
                                        div { class: "capture-calendar-grid",
                                            for cell in calendar_cells {
                                                if let Some(display) = cell.display.clone() {
                                                    button {
                                                        class: if cell.is_selected {
                                                            "capture-calendar-day capture-calendar-day-selected"
                                                        } else if cell.is_today {
                                                            "capture-calendar-day capture-calendar-day-today"
                                                        } else {
                                                            "capture-calendar-day"
                                                        },
                                                        tabindex: "-1",
                                                        onclick: move |_| sync_due_input(display.clone()),
                                                        "{cell.label}"
                                                    }
                                                } else {
                                                    span { class: "capture-calendar-day capture-calendar-day-empty", "" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if step() == CaptureState::Details(DetailStep::Priority) {
                                div {
                                    id: "capture-priority-section",
                                    class: "capture-section capture-section-active",
                                    p { class: "capture-label", "Priority" }
                                    div { class: "capture-suggestions",
                                        for option in ['H', 'M', 'L'] {
                                            button {
                                                class: if draft.read().priority == Some(option) {
                                                    "capture-chip capture-chip-selected"
                                                } else {
                                                    "capture-chip"
                                                },
                                                tabindex: "-1",
                                                onclick: move |_| draft.with_mut(|draft| draft.priority = Some(option)),
                                                "{option}"
                                            }
                                        }
                                    }
                                    p {
                                        class: "capture-supporting-copy",
                                        if draft.read().due_normalized.is_some() {
                                            "Required because a due date is set."
                                        } else {
                                            "Optional without a due date."
                                        }
                                    }
                                    input {
                                        id: "capture-priority-input",
                                        class: "capture-input capture-input-detail capture-input-compact",
                                        value: "{draft.read().priority.map(|value| value.to_string()).unwrap_or_default()}",
                                        placeholder: "H, M, or L",
                                        autofocus: true,
                                        onfocus: move |_| step.set(CaptureState::Details(DetailStep::Priority)),
                                        oninput: move |event| {
                                            let next = event.value().chars().next().map(|value| value.to_ascii_uppercase());
                                            draft.with_mut(|draft| {
                                                draft.priority = next.filter(|value| matches!(value, 'H' | 'M' | 'L'));
                                            });
                                        },
                                        onkeydown: move |event| {
                                            if step() != CaptureState::Details(DetailStep::Priority) {
                                                return;
                                            }

                                            match event.key() {
                                                Key::ArrowLeft | Key::ArrowUp => {
                                                    event.prevent_default();
                                                    let next = previous_priority(draft.read().priority);
                                                    draft.with_mut(|draft| draft.priority = Some(next));
                                                }
                                                Key::ArrowRight | Key::ArrowDown => {
                                                    event.prevent_default();
                                                    let next = next_priority(draft.read().priority);
                                                    draft.with_mut(|draft| draft.priority = Some(next));
                                                }
                                                Key::Enter => {
                                                    event.prevent_default();
                                                    if draft.read().priority.is_none() {
                                                        draft.with_mut(|draft| draft.priority = Some('M'));
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }

                            if step() == CaptureState::Details(DetailStep::Tags) {
                                div {
                                    id: "capture-tags-section",
                                    class: "capture-section capture-section-active",
                                    p { class: "capture-label", "Tags" }
                                    input {
                                        id: "capture-tags-input",
                                        class: "capture-input capture-input-detail",
                                        value: "{draft.read().tags_input}",
                                        placeholder: "#idea #writing release",
                                        autofocus: true,
                                        onfocus: move |_| step.set(CaptureState::Details(DetailStep::Tags)),
                                        oninput: move |event| {
                                            tag_selection.set(0);
                                            draft.with_mut(|draft| draft.tags_input = event.value());
                                        },
                                        onkeydown: move |event| {
                                            if step() != CaptureState::Details(DetailStep::Tags) {
                                                return;
                                            }

                                            match event.key() {
                                                Key::ArrowRight | Key::ArrowDown => {
                                                    event.prevent_default();
                                                    if !tag_suggestions_for_keys.is_empty() {
                                                        let next = (tag_selection() + 1)
                                                            .min(tag_suggestions_for_keys.len().saturating_sub(1));
                                                        tag_selection.set(next);
                                                    }
                                                }
                                                Key::ArrowLeft | Key::ArrowUp => {
                                                    event.prevent_default();
                                                    tag_selection.set(tag_selection().saturating_sub(1));
                                                }
                                                Key::Enter => {
                                                    event.prevent_default();
                                                    let merged = apply_selected_suggestion(
                                                        tag_selection(),
                                                        &tag_suggestions_for_keys,
                                                        &draft.read().tags_input,
                                                        SuggestionKind::Tag,
                                                    );
                                                    draft.with_mut(|draft| draft.tags_input = merged);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    if !tag_suggestions.is_empty() {
                                        p { class: "capture-supporting-copy", "Arrow keys select suggestions. Enter inserts the highlighted tag." }
                                        div { class: "capture-suggestions",
                                            for (index, item) in tag_suggestions_for_list.clone().into_iter().enumerate() {
                                                button {
                                                    class: if index == tag_selection() { "capture-chip capture-chip-selected" } else { "capture-chip" },
                                                    tabindex: "-1",
                                                    onclick: move |_| {
                                                        tag_selection.set(index);
                                                        let merged = merge_tag_input(&draft.read().tags_input, Some(&item));
                                                        draft.with_mut(|draft| draft.tags_input = merged);
                                                    },
                                                    "#{item}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !toast_msg().is_empty() {
                    p {
                        class: if toast_is_error() { "capture-toast capture-toast-error" } else { "capture-toast" },
                        "{toast_msg}"
                    }
                }

                div {
                    class: "capture-footer",
                    p {
                        class: "capture-hints",
                        if step() == CaptureState::Details(DetailStep::Tags) {
                            "Ctrl+D submits. Esc steps back."
                        } else {
                            "Ctrl+D continues. Esc steps back."
                        }
                    }
                }
            }
        }
    }
}
