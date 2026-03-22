use std::collections::HashMap;

use devprune_core::config::AppPaths;
use devprune_core::rules::types::{Category, SafetyLevel};
use devprune_core::trash::metadata::TrashManifestEntry;
use devprune_core::trash::storage::TrashManager;
use devprune_core::types::ArtifactInfo;
use uuid::Uuid;

// ── Sort / filter state ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    SizeDesc,
    Name,
    Path,
}

impl SortOrder {
    pub fn next(self) -> Self {
        match self {
            SortOrder::SizeDesc => SortOrder::Name,
            SortOrder::Name => SortOrder::Path,
            SortOrder::Path => SortOrder::SizeDesc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortOrder::SizeDesc => "size↓",
            SortOrder::Name => "name",
            SortOrder::Path => "path",
        }
    }
}

// ── Safety filter ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SafetyFilter {
    #[default]
    All,
    Safe,
    Cautious,
    Risky,
}

impl SafetyFilter {
    pub fn next(self) -> Self {
        match self {
            SafetyFilter::All => SafetyFilter::Safe,
            SafetyFilter::Safe => SafetyFilter::Cautious,
            SafetyFilter::Cautious => SafetyFilter::Risky,
            SafetyFilter::Risky => SafetyFilter::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SafetyFilter::All => "all",
            SafetyFilter::Safe => "safe only",
            SafetyFilter::Cautious => "cautious only",
            SafetyFilter::Risky => "risky only",
        }
    }

    pub fn is_active(self) -> bool {
        !matches!(self, SafetyFilter::All)
    }

    /// Returns true when the given safety level should be visible under this
    /// filter.
    pub fn matches(self, level: SafetyLevel) -> bool {
        match self {
            SafetyFilter::All => true,
            SafetyFilter::Safe => level == SafetyLevel::Safe,
            SafetyFilter::Cautious => level == SafetyLevel::Cautious,
            SafetyFilter::Risky => level == SafetyLevel::Risky,
        }
    }
}

// ── Check state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Checked,
    Unchecked,
    Indeterminate,
}

// ── Tree nodes ────────────────────────────────────────────────────────────────

/// Level 0 – category header.
#[derive(Debug, Clone)]
pub struct CategoryNode {
    pub category: Category,
    pub expanded: bool,
    pub check_state: CheckState,
    pub children: Vec<RuleGroupNode>,
    /// Aggregate size of all visible children.
    pub total_size: u64,
}

/// Level 1 – rule group (all artifacts from the same rule).
#[derive(Debug, Clone)]
pub struct RuleGroupNode {
    pub rule_id: String,
    pub rule_name: String,
    #[allow(dead_code)]
    pub category: Category,
    pub expanded: bool,
    pub check_state: CheckState,
    pub children: Vec<ArtifactNode>,
    /// Aggregate size of children.
    pub total_size: u64,
}

/// Level 2 – individual artifact.
#[derive(Debug, Clone)]
pub struct ArtifactNode {
    pub artifact: ArtifactInfo,
    pub checked: bool,
}

impl ArtifactNode {
    pub fn size(&self) -> u64 {
        self.artifact.size.unwrap_or(0)
    }
}

// ── Flattened row (for rendering) ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RowRef {
    Category { cat_idx: usize },
    RuleGroup { cat_idx: usize, grp_idx: usize },
    Artifact { cat_idx: usize, grp_idx: usize, art_idx: usize },
}

#[derive(Debug, Clone)]
pub struct VisibleRow {
    pub row_ref: RowRef,
    pub depth: u8,
    pub check_state: CheckState,
    pub expanded: Option<bool>,
    pub name: String,
    pub size: u64,
    pub item_count: Option<usize>,
}

// ── TreeState ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct TreeState {
    pub categories: Vec<CategoryNode>,
    /// Maps artifact Uuid to (cat_idx, grp_idx, art_idx) for O(1) size updates.
    index: HashMap<Uuid, (usize, usize, usize)>,
    pub cursor: usize,
    pub sort: SortOrder,
    pub search_filter: Option<String>,
    pub safety_filter: SafetyFilter,
}

impl TreeState {
    pub fn add_artifact(&mut self, artifact: ArtifactInfo) {
        let category = artifact.category;
        let rule_id = artifact.rule_id.clone();
        let rule_name = artifact.rule_name.clone();
        let artifact_id = artifact.id;

        // Find or create category node.
        let cat_idx = if let Some(idx) = self.categories.iter().position(|c| c.category == category) {
            idx
        } else {
            let node = CategoryNode {
                category,
                expanded: true,
                check_state: CheckState::Unchecked,
                children: Vec::new(),
                total_size: 0,
            };
            self.categories.push(node);
            self.categories.len() - 1
        };

        // Find or create rule group node.
        let grp_idx = if let Some(idx) = self.categories[cat_idx]
            .children
            .iter()
            .position(|g| g.rule_id == rule_id)
        {
            idx
        } else {
            let node = RuleGroupNode {
                rule_id,
                rule_name,
                category,
                expanded: true,
                check_state: CheckState::Unchecked,
                children: Vec::new(),
                total_size: 0,
            };
            self.categories[cat_idx].children.push(node);
            self.categories[cat_idx].children.len() - 1
        };

        let art_idx = self.categories[cat_idx].children[grp_idx].children.len();
        self.index.insert(artifact_id, (cat_idx, grp_idx, art_idx));

        let size = artifact.size.unwrap_or(0);
        self.categories[cat_idx].children[grp_idx]
            .children
            .push(ArtifactNode { artifact, checked: false });

        // Update aggregate sizes.
        self.categories[cat_idx].children[grp_idx].total_size += size;
        self.categories[cat_idx].total_size += size;
    }

    pub fn update_size(&mut self, id: Uuid, size: u64) {
        if let Some(&(ci, gi, ai)) = self.index.get(&id) {
            let art = &mut self.categories[ci].children[gi].children[ai];
            let old = art.artifact.size.unwrap_or(0);
            art.artifact.size = Some(size);
            let delta = size.saturating_sub(old);
            self.categories[ci].children[gi].total_size += delta;
            self.categories[ci].total_size += delta;
        }
    }

    /// Returns a flattened, sorted list of visible rows.
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = Vec::new();
        let mut cats: Vec<(usize, &CategoryNode)> = self.categories.iter().enumerate().collect();
        cats.sort_by_key(|(_, c)| c.category.display_name());

        for (ci, cat) in &cats {
            let ci = *ci;
            // Skip category if no child passes all active filters.
            let cat_has_visible = cat.children.iter().any(|g| {
                g.children.iter().any(|a| self.artifact_is_visible(a))
            });
            if !cat_has_visible {
                continue;
            }

            rows.push(VisibleRow {
                row_ref: RowRef::Category { cat_idx: ci },
                depth: 0,
                check_state: cat.check_state,
                expanded: Some(cat.expanded),
                name: cat.category.display_name().to_string(),
                size: cat.total_size,
                item_count: Some(cat.children.iter().map(|g| g.children.len()).sum()),
            });

            if !cat.expanded {
                continue;
            }

            let mut groups: Vec<(usize, &RuleGroupNode)> =
                cat.children.iter().enumerate().collect();
            // Sort groups according to current sort order.
            match self.sort {
                SortOrder::SizeDesc => groups.sort_by(|a, b| b.1.total_size.cmp(&a.1.total_size)),
                SortOrder::Name => groups.sort_by_key(|(_, g)| g.rule_name.as_str()),
                SortOrder::Path => groups.sort_by_key(|(_, g)| g.rule_id.as_str()),
            }

            for (gi, grp) in &groups {
                let gi = *gi;
                let grp_has_visible = grp.children.iter().any(|a| self.artifact_is_visible(a));
                if !grp_has_visible {
                    continue;
                }

                rows.push(VisibleRow {
                    row_ref: RowRef::RuleGroup { cat_idx: ci, grp_idx: gi },
                    depth: 1,
                    check_state: grp.check_state,
                    expanded: Some(grp.expanded),
                    name: grp.rule_name.clone(),
                    size: grp.total_size,
                    item_count: Some(grp.children.len()),
                });

                if !grp.expanded {
                    continue;
                }

                let mut arts: Vec<(usize, &ArtifactNode)> =
                    grp.children.iter().enumerate().collect();
                match self.sort {
                    SortOrder::SizeDesc => {
                        arts.sort_by(|a, b| b.1.size().cmp(&a.1.size()))
                    }
                    SortOrder::Name => arts.sort_by_key(|(_, a)| {
                        a.artifact
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    }),
                    SortOrder::Path => arts.sort_by_key(|(_, a)| a.artifact.path.clone()),
                }

                for (ai, art) in &arts {
                    let ai = *ai;
                    if !self.artifact_is_visible(art) {
                        continue;
                    }

                    let name = art
                        .artifact
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| art.artifact.path.display().to_string());

                    rows.push(VisibleRow {
                        row_ref: RowRef::Artifact { cat_idx: ci, grp_idx: gi, art_idx: ai },
                        depth: 2,
                        check_state: if art.checked {
                            CheckState::Checked
                        } else {
                            CheckState::Unchecked
                        },
                        expanded: None,
                        name,
                        size: art.size(),
                        item_count: None,
                    });
                }
            }
        }
        rows
    }

    /// Returns true when the artifact should be shown given the current
    /// search filter and safety filter.
    fn artifact_is_visible(&self, art: &ArtifactNode) -> bool {
        if !self.safety_filter.matches(art.artifact.safety) {
            return false;
        }
        if let Some(ref q) = self.search_filter {
            let q = q.to_lowercase();
            if !art.artifact.path.to_string_lossy().to_lowercase().contains(&q)
                && !art.artifact.rule_name.to_lowercase().contains(&q)
            {
                return false;
            }
        }
        true
    }

    pub fn move_cursor(&mut self, delta: i64) {
        let count = self.visible_rows().len();
        if count == 0 {
            return;
        }
        if delta > 0 {
            self.cursor = (self.cursor + delta as usize).min(count - 1);
        } else {
            let back = (-delta) as usize;
            self.cursor = self.cursor.saturating_sub(back);
        }
    }

    pub fn toggle_check(&mut self, cursor: usize) {
        let rows = self.visible_rows();
        if cursor >= rows.len() {
            return;
        }
        match rows[cursor].row_ref.clone() {
            RowRef::Category { cat_idx } => {
                let new_checked = self.categories[cat_idx].check_state != CheckState::Checked;
                let new_state = if new_checked {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                for grp in &mut self.categories[cat_idx].children {
                    for art in &mut grp.children {
                        art.checked = new_checked;
                    }
                    grp.check_state = new_state;
                }
                self.categories[cat_idx].check_state = new_state;
            }
            RowRef::RuleGroup { cat_idx, grp_idx } => {
                let grp = &mut self.categories[cat_idx].children[grp_idx];
                let new_checked = grp.check_state != CheckState::Checked;
                for art in &mut grp.children {
                    art.checked = new_checked;
                }
                grp.check_state = if new_checked {
                    CheckState::Checked
                } else {
                    CheckState::Unchecked
                };
                self.refresh_category_check(cat_idx);
            }
            RowRef::Artifact { cat_idx, grp_idx, art_idx } => {
                let art = &mut self.categories[cat_idx].children[grp_idx].children[art_idx];
                art.checked = !art.checked;
                self.refresh_group_check(cat_idx, grp_idx);
                self.refresh_category_check(cat_idx);
            }
        }
    }

    pub fn toggle_expand(&mut self, cursor: usize) {
        let rows = self.visible_rows();
        if cursor >= rows.len() {
            return;
        }
        match rows[cursor].row_ref.clone() {
            RowRef::Category { cat_idx } => {
                self.categories[cat_idx].expanded = !self.categories[cat_idx].expanded;
            }
            RowRef::RuleGroup { cat_idx, grp_idx } => {
                self.categories[cat_idx].children[grp_idx].expanded =
                    !self.categories[cat_idx].children[grp_idx].expanded;
            }
            RowRef::Artifact { .. } => {}
        }
    }

    /// Expand the node at cursor. If already expanded or a leaf, no-op.
    pub fn expand(&mut self, cursor: usize) {
        let rows = self.visible_rows();
        if cursor >= rows.len() {
            return;
        }
        match rows[cursor].row_ref.clone() {
            RowRef::Category { cat_idx } => {
                self.categories[cat_idx].expanded = true;
            }
            RowRef::RuleGroup { cat_idx, grp_idx } => {
                self.categories[cat_idx].children[grp_idx].expanded = true;
            }
            RowRef::Artifact { .. } => {}
        }
    }

    /// Collapse the node at cursor. If on a leaf, collapse the parent instead.
    pub fn collapse(&mut self, cursor: usize) {
        let rows = self.visible_rows();
        if cursor >= rows.len() {
            return;
        }
        match rows[cursor].row_ref.clone() {
            RowRef::Category { cat_idx } => {
                self.categories[cat_idx].expanded = false;
            }
            RowRef::RuleGroup { cat_idx, grp_idx } => {
                if self.categories[cat_idx].children[grp_idx].expanded {
                    self.categories[cat_idx].children[grp_idx].expanded = false;
                } else {
                    // Already collapsed - jump cursor to parent category
                    // Find the category row in visible rows
                    for (i, r) in rows.iter().enumerate() {
                        if matches!(&r.row_ref, RowRef::Category { cat_idx: ci } if *ci == cat_idx) {
                            self.cursor = i;
                            break;
                        }
                    }
                }
            }
            RowRef::Artifact { cat_idx, grp_idx, .. } => {
                // On a leaf - collapse the parent rule group
                self.categories[cat_idx].children[grp_idx].expanded = false;
                // Jump cursor to the rule group row
                for (i, r) in rows.iter().enumerate() {
                    if matches!(&r.row_ref, RowRef::RuleGroup { cat_idx: ci, grp_idx: gi } if *ci == cat_idx && *gi == grp_idx) {
                        self.cursor = i;
                        break;
                    }
                }
            }
        }
    }

    pub fn select_all(&mut self, checked: bool) {
        let state = if checked { CheckState::Checked } else { CheckState::Unchecked };
        for cat in &mut self.categories {
            for grp in &mut cat.children {
                for art in &mut grp.children {
                    art.checked = checked;
                }
                grp.check_state = state;
            }
            cat.check_state = state;
        }
    }

    /// Returns all currently checked artifacts.
    pub fn selected_artifacts(&self) -> Vec<&ArtifactInfo> {
        self.categories
            .iter()
            .flat_map(|c| c.children.iter())
            .flat_map(|g| g.children.iter())
            .filter(|a| a.checked)
            .map(|a| &a.artifact)
            .collect()
    }

    /// Returns the artifact at the given cursor position, if it is an artifact row.
    pub fn cursor_artifact(&self) -> Option<&ArtifactInfo> {
        let rows = self.visible_rows();
        let row = rows.get(self.cursor)?;
        if let RowRef::Artifact { cat_idx, grp_idx, art_idx } = &row.row_ref {
            Some(&self.categories[*cat_idx].children[*grp_idx].children[*art_idx].artifact)
        } else {
            None
        }
    }

    /// Removes all checked artifacts from the tree (used for the stub delete action).
    pub fn remove_checked(&mut self) {
        for cat in &mut self.categories {
            for grp in &mut cat.children {
                let before: u64 = grp.children.iter().map(|a| a.size()).sum();
                grp.children.retain(|a| !a.checked);
                let after: u64 = grp.children.iter().map(|a| a.size()).sum();
                grp.total_size = grp.total_size.saturating_sub(before - after);
            }
            cat.children.retain(|g| !g.children.is_empty());
            cat.total_size = cat.children.iter().map(|g| g.total_size).sum();
        }
        self.categories.retain(|c| !c.children.is_empty());

        // Refresh check states on all remaining nodes so no stale [~] lingers.
        let cat_count = self.categories.len();
        for ci in 0..cat_count {
            let grp_count = self.categories[ci].children.len();
            for gi in 0..grp_count {
                self.refresh_group_check(ci, gi);
            }
            self.refresh_category_check(ci);
        }

        // Rebuild the index.
        self.index.clear();
        for (ci, cat) in self.categories.iter().enumerate() {
            for (gi, grp) in cat.children.iter().enumerate() {
                for (ai, art) in grp.children.iter().enumerate() {
                    self.index.insert(art.artifact.id, (ci, gi, ai));
                }
            }
        }
        // Clamp cursor.
        let count = self.visible_rows().len();
        if count == 0 {
            self.cursor = 0;
        } else {
            self.cursor = self.cursor.min(count - 1);
        }
    }

    pub fn selection_summary(&self) -> (usize, u64) {
        let arts = self.selected_artifacts();
        let count = arts.len();
        let size: u64 = arts.iter().map(|a| a.size.unwrap_or(0)).sum();
        (count, size)
    }

    fn refresh_group_check(&mut self, cat_idx: usize, grp_idx: usize) {
        let grp = &mut self.categories[cat_idx].children[grp_idx];
        let checked = grp.children.iter().filter(|a| a.checked).count();
        grp.check_state = if checked == 0 {
            CheckState::Unchecked
        } else if checked == grp.children.len() {
            CheckState::Checked
        } else {
            CheckState::Indeterminate
        };
    }

    fn refresh_category_check(&mut self, cat_idx: usize) {
        let cat = &mut self.categories[cat_idx];
        let checked = cat
            .children
            .iter()
            .filter(|g| g.check_state == CheckState::Checked)
            .count();
        let partial = cat
            .children
            .iter()
            .any(|g| g.check_state == CheckState::Indeterminate);
        cat.check_state = if checked == cat.children.len() && !cat.children.is_empty() {
            CheckState::Checked
        } else if checked == 0 && !partial {
            CheckState::Unchecked
        } else {
            CheckState::Indeterminate
        };
    }
}

// ── Scan progress ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ScanProgressState {
    pub dirs_visited: u64,
    pub artifacts_found: u64,
    pub elapsed_ms: u64,
}

// ── App mode ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search { query: String },
    ConfirmDelete,
    Help,
    TrashBrowser,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub tree: TreeState,
    pub mode: AppMode,
    pub scan_complete: bool,
    pub scan_progress: ScanProgressState,
    pub should_quit: bool,
    #[allow(dead_code)]
    pub app_paths: AppPaths,
    /// Incremented each tick to drive the spinner animation.
    pub tick_count: u64,
    /// Non-fatal scan warnings.
    pub scan_errors: Vec<String>,
    /// Brief status message shown after an operation completes.
    pub status_message: Option<String>,
    /// Trash manager for moving items to the devprune trash.
    pub trash_manager: Option<TrashManager>,
    /// State for the trash browser overlay.
    pub trash_browser: TrashBrowserState,
    /// Cached trash stats (item count, total bytes).
    pub trash_stats: TrashStats,
}

#[derive(Debug, Clone, Default)]
pub struct TrashStats {
    pub item_count: usize,
    pub total_bytes: u64,
}

impl App {
    pub fn new(app_paths: AppPaths, trash_manager: Option<TrashManager>) -> Self {
        let trash_stats = trash_manager
            .as_ref()
            .and_then(|tm| tm.list_items().ok())
            .map(|items| TrashStats {
                item_count: items.len(),
                total_bytes: items.iter().map(|i| i.size_bytes).sum(),
            })
            .unwrap_or_default();

        Self {
            tree: TreeState::default(),
            mode: AppMode::Normal,
            scan_complete: false,
            scan_progress: ScanProgressState::default(),
            should_quit: false,
            app_paths,
            tick_count: 0,
            scan_errors: Vec::new(),
            status_message: None,
            trash_manager,
            trash_browser: TrashBrowserState::default(),
            trash_stats,
        }
    }

    /// Refresh cached trash stats from the trash manager.
    pub fn refresh_trash_stats(&mut self) {
        self.trash_stats = self
            .trash_manager
            .as_ref()
            .and_then(|tm| tm.list_items().ok())
            .map(|items| TrashStats {
                item_count: items.len(),
                total_bytes: items.iter().map(|i| i.size_bytes).sum(),
            })
            .unwrap_or_default();
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }
}

// ── TrashBrowserState ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrashSort {
    #[default]
    DateDesc,
    DateAsc,
    SizeDesc,
    SizeAsc,
}

impl TrashSort {
    pub fn next(self) -> Self {
        match self {
            Self::DateDesc => Self::SizeDesc,
            Self::SizeDesc => Self::SizeAsc,
            Self::SizeAsc => Self::DateDesc,
            Self::DateAsc => Self::DateDesc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DateDesc => "newest first",
            Self::DateAsc => "oldest first",
            Self::SizeDesc => "largest first",
            Self::SizeAsc => "smallest first",
        }
    }
}

#[derive(Debug, Default)]
pub struct TrashBrowserState {
    pub items: Vec<TrashManifestEntry>,
    pub cursor: usize,
    pub checked: Vec<bool>,
    pub sort: TrashSort,
}

impl TrashBrowserState {
    pub fn load(&mut self, mut items: Vec<TrashManifestEntry>) {
        self.apply_sort(&mut items);
        let len = items.len();
        self.items = items;
        self.checked = vec![false; len];
        self.cursor = 0;
    }

    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        let mut items = std::mem::take(&mut self.items);
        self.apply_sort(&mut items);
        // Preserve checked state by rebuilding from ids
        let checked_ids: std::collections::HashSet<uuid::Uuid> = self.items.iter()
            .enumerate()
            .filter(|(i, _)| self.checked.get(*i).copied().unwrap_or(false))
            .map(|(_, e)| e.id)
            .collect();
        let len = items.len();
        self.items = items;
        self.checked = self.items.iter().map(|e| checked_ids.contains(&e.id)).collect();
        if len > 0 {
            self.cursor = self.cursor.min(len - 1);
        }
    }

    fn apply_sort(&self, items: &mut Vec<TrashManifestEntry>) {
        match self.sort {
            TrashSort::DateDesc => items.sort_by(|a, b| b.trashed_at.cmp(&a.trashed_at)),
            TrashSort::DateAsc => items.sort_by(|a, b| a.trashed_at.cmp(&b.trashed_at)),
            TrashSort::SizeDesc => items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            TrashSort::SizeAsc => items.sort_by(|a, b| a.size_bytes.cmp(&b.size_bytes)),
        }
    }

    pub fn move_cursor(&mut self, delta: i64) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        if delta > 0 {
            self.cursor = (self.cursor + delta as usize).min(count - 1);
        } else {
            self.cursor = self.cursor.saturating_sub((-delta) as usize);
        }
    }

    pub fn toggle_check(&mut self) {
        if let Some(v) = self.checked.get_mut(self.cursor) {
            *v = !*v;
        }
    }

    pub fn selected_ids(&self) -> Vec<uuid::Uuid> {
        self.items
            .iter()
            .enumerate()
            .filter(|(i, _)| self.checked.get(*i).copied().unwrap_or(false))
            .map(|(_, e)| e.id)
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use devprune_core::rules::types::{Category, SafetyLevel};
    use devprune_core::types::ArtifactInfo;
    use uuid::Uuid;

    use super::*;

    fn make_artifact(path: &str, rule_id: &str, category: Category) -> ArtifactInfo {
        ArtifactInfo {
            id: Uuid::new_v4(),
            path: PathBuf::from(path),
            rule_id: rule_id.to_string(),
            rule_name: rule_id.to_string(),
            category,
            safety: SafetyLevel::Safe,
            size: Some(1024),
            last_modified: None,
            is_directory: true,
        }
    }

    #[test]
    fn add_artifact_creates_category_and_group() {
        let mut tree = TreeState::default();
        let art = make_artifact("/tmp/node_modules", "npm", Category::Dependencies);
        tree.add_artifact(art);
        assert_eq!(tree.categories.len(), 1);
        assert_eq!(tree.categories[0].children.len(), 1);
        assert_eq!(tree.categories[0].children[0].children.len(), 1);
    }

    #[test]
    fn add_artifact_reuses_existing_category_and_group() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/a/node_modules", "npm", Category::Dependencies));
        tree.add_artifact(make_artifact("/tmp/b/node_modules", "npm", Category::Dependencies));
        assert_eq!(tree.categories.len(), 1);
        assert_eq!(tree.categories[0].children.len(), 1);
        assert_eq!(tree.categories[0].children[0].children.len(), 2);
    }

    #[test]
    fn update_size_propagates_to_aggregates() {
        let mut tree = TreeState::default();
        let art = make_artifact("/tmp/node_modules", "npm", Category::Dependencies);
        let id = art.id;
        tree.add_artifact(art);
        tree.update_size(id, 4096);
        assert_eq!(tree.categories[0].children[0].total_size, 4096);
        assert_eq!(tree.categories[0].total_size, 4096);
    }

    #[test]
    fn toggle_check_artifact_propagates_to_group_and_category() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/node_modules", "npm", Category::Dependencies));
        tree.toggle_check(2); // row 0=cat, 1=grp, 2=artifact
        let art = &tree.categories[0].children[0].children[0];
        assert!(art.checked);
        assert_eq!(tree.categories[0].children[0].check_state, CheckState::Checked);
        assert_eq!(tree.categories[0].check_state, CheckState::Checked);
    }

    #[test]
    fn select_all_and_deselect_all() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/a", "npm", Category::Dependencies));
        tree.add_artifact(make_artifact("/tmp/b", "cargo", Category::BuildOutput));
        tree.select_all(true);
        assert_eq!(tree.selected_artifacts().len(), 2);
        tree.select_all(false);
        assert_eq!(tree.selected_artifacts().len(), 0);
    }

    #[test]
    fn remove_checked_cleans_tree() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/a", "npm", Category::Dependencies));
        tree.add_artifact(make_artifact("/tmp/b", "npm", Category::Dependencies));
        tree.toggle_check(2); // check first artifact (rows: 0=cat, 1=grp, 2=art0, 3=art1)
        tree.remove_checked();
        assert_eq!(tree.categories[0].children[0].children.len(), 1);
    }

    #[test]
    fn visible_rows_respects_collapse() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/a", "npm", Category::Dependencies));
        let rows_expanded = tree.visible_rows().len();
        tree.collapse(0); // collapse category
        let rows_collapsed = tree.visible_rows().len();
        assert!(rows_expanded > rows_collapsed);
    }

    #[test]
    fn search_filter_hides_non_matching() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact("/tmp/node_modules", "npm", Category::Dependencies));
        tree.add_artifact(make_artifact("/tmp/target", "cargo", Category::BuildOutput));
        tree.search_filter = Some("node".to_string());
        let rows = tree.visible_rows();
        // Should show: Dependencies category, npm group, artifact = 3 rows.
        // BuildOutput category should be hidden because no child matches "node".
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn sort_order_cycles() {
        assert_eq!(SortOrder::SizeDesc.next(), SortOrder::Name);
        assert_eq!(SortOrder::Name.next(), SortOrder::Path);
        assert_eq!(SortOrder::Path.next(), SortOrder::SizeDesc);
    }

    fn make_artifact_with_safety(path: &str, rule_id: &str, category: Category, safety: SafetyLevel) -> ArtifactInfo {
        ArtifactInfo {
            safety,
            ..make_artifact(path, rule_id, category)
        }
    }

    #[test]
    fn safety_filter_hides_non_matching() {
        let mut tree = TreeState::default();
        tree.add_artifact(make_artifact_with_safety("/tmp/a", "npm", Category::Dependencies, SafetyLevel::Safe));
        tree.add_artifact(make_artifact_with_safety("/tmp/b", "cargo", Category::BuildOutput, SafetyLevel::Cautious));
        tree.add_artifact(make_artifact_with_safety("/tmp/c", "venv", Category::VirtualEnv, SafetyLevel::Risky));

        tree.safety_filter = SafetyFilter::Safe;
        let rows = tree.visible_rows();
        // Should show: Dependencies category, npm group, /tmp/a = 3 rows.
        assert_eq!(rows.len(), 3, "expected 3 rows for safe filter, got {}", rows.len());

        tree.safety_filter = SafetyFilter::Cautious;
        let rows = tree.visible_rows();
        assert_eq!(rows.len(), 3);

        tree.safety_filter = SafetyFilter::All;
        let rows = tree.visible_rows();
        // All 3 categories, 3 groups, 3 artifacts = 9 rows.
        assert_eq!(rows.len(), 9);
    }

    #[test]
    fn safety_filter_cycles() {
        assert_eq!(SafetyFilter::All.next(), SafetyFilter::Safe);
        assert_eq!(SafetyFilter::Safe.next(), SafetyFilter::Cautious);
        assert_eq!(SafetyFilter::Cautious.next(), SafetyFilter::Risky);
        assert_eq!(SafetyFilter::Risky.next(), SafetyFilter::All);
    }
}
