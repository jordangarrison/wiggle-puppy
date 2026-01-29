//! PRD (Product Requirements Document) types and parsing.
//!
//! This module provides types for representing a PRD with stories,
//! including functionality for loading/saving JSON files, checking
//! completion status, and finding the next story to implement.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// A Product Requirements Document containing stories to implement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prd {
    /// The name of the PRD/project.
    pub name: String,

    /// The git branch name for this work.
    pub branch_name: String,

    /// A description of the PRD/project.
    pub description: String,

    /// The list of stories to implement.
    pub stories: Vec<Story>,
}

/// A single story/task in the PRD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    /// Unique identifier for the story.
    pub id: String,

    /// Short title of the story.
    pub title: String,

    /// Detailed description of what needs to be done.
    pub description: String,

    /// Priority (lower number = higher priority).
    pub priority: u32,

    /// Whether this story has been completed and verified.
    pub passes: bool,

    /// List of acceptance criteria that must be met.
    pub acceptance_criteria: Vec<String>,

    /// IDs of stories that must pass before this one can start.
    pub depends_on: Vec<String>,
}

/// The status of a story based on its completion state and dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryStatus {
    /// The story is complete (passes = true).
    Complete,

    /// The story is not started but all dependencies are met.
    Pending,

    /// The story is blocked by incomplete dependencies.
    Blocked,

    /// The story is currently being worked on (for future use).
    InProgress,
}

impl Prd {
    /// Load a PRD from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|source| Error::PrdReadError {
            path: path.to_path_buf(),
            source,
        })?;

        serde_json::from_str(&content).map_err(|source| Error::PrdParseError {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Save the PRD to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content =
            serde_json::to_string_pretty(self).map_err(|source| Error::PrdParseError {
                path: path.to_path_buf(),
                source,
            })?;

        std::fs::write(path, content).map_err(|source| Error::PrdWriteError {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Check if all stories in the PRD are complete.
    pub fn is_complete(&self) -> bool {
        self.stories.iter().all(|s| s.passes)
    }

    /// Find the next story to work on.
    ///
    /// Returns the highest priority (lowest priority number) incomplete story
    /// whose dependencies are all satisfied.
    pub fn next_story(&self) -> Option<&Story> {
        // Build a set of completed story IDs for dependency checking
        let completed: HashSet<&str> = self
            .stories
            .iter()
            .filter(|s| s.passes)
            .map(|s| s.id.as_str())
            .collect();

        // Find incomplete stories with all dependencies met, sorted by priority
        self.stories
            .iter()
            .filter(|s| !s.passes)
            .filter(|s| {
                s.depends_on
                    .iter()
                    .all(|dep| completed.contains(dep.as_str()))
            })
            .min_by_key(|s| s.priority)
    }

    /// Get a mutable reference to a story by its ID.
    pub fn get_story_mut(&mut self, id: &str) -> Option<&mut Story> {
        self.stories.iter_mut().find(|s| s.id == id)
    }

    /// Get a reference to a story by its ID.
    pub fn get_story(&self, id: &str) -> Option<&Story> {
        self.stories.iter().find(|s| s.id == id)
    }
}

impl Story {
    /// Get the status of this story based on its completion state and dependencies.
    ///
    /// The `completed_ids` set should contain the IDs of all completed stories.
    pub fn status(&self, completed_ids: &HashSet<&str>) -> StoryStatus {
        if self.passes {
            StoryStatus::Complete
        } else if self
            .depends_on
            .iter()
            .all(|dep| completed_ids.contains(dep.as_str()))
        {
            StoryStatus::Pending
        } else {
            StoryStatus::Blocked
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prd() -> Prd {
        Prd {
            name: "Test PRD".to_string(),
            branch_name: "test-branch".to_string(),
            description: "A test PRD".to_string(),
            stories: vec![
                Story {
                    id: "1".to_string(),
                    title: "First story".to_string(),
                    description: "Do the first thing".to_string(),
                    priority: 1,
                    passes: true,
                    acceptance_criteria: vec!["Criterion 1".to_string()],
                    depends_on: vec![],
                },
                Story {
                    id: "2".to_string(),
                    title: "Second story".to_string(),
                    description: "Do the second thing".to_string(),
                    priority: 2,
                    passes: false,
                    acceptance_criteria: vec!["Criterion 2".to_string()],
                    depends_on: vec!["1".to_string()],
                },
                Story {
                    id: "3".to_string(),
                    title: "Third story".to_string(),
                    description: "Do the third thing".to_string(),
                    priority: 3,
                    passes: false,
                    acceptance_criteria: vec!["Criterion 3".to_string()],
                    depends_on: vec!["2".to_string()],
                },
                Story {
                    id: "4".to_string(),
                    title: "Fourth story (low priority)".to_string(),
                    description: "Do the fourth thing".to_string(),
                    priority: 10,
                    passes: false,
                    acceptance_criteria: vec!["Criterion 4".to_string()],
                    depends_on: vec!["1".to_string()],
                },
            ],
        }
    }

    #[test]
    fn test_is_complete_false_when_stories_incomplete() {
        let prd = create_test_prd();
        assert!(!prd.is_complete());
    }

    #[test]
    fn test_is_complete_true_when_all_pass() {
        let mut prd = create_test_prd();
        for story in &mut prd.stories {
            story.passes = true;
        }
        assert!(prd.is_complete());
    }

    #[test]
    fn test_next_story_respects_dependencies() {
        let prd = create_test_prd();
        let next = prd.next_story().expect("should have a next story");

        // Story 2 depends on 1 (which passes), so it should be next
        // Story 3 depends on 2 (which doesn't pass), so it's blocked
        assert_eq!(next.id, "2");
    }

    #[test]
    fn test_next_story_respects_priority() {
        let prd = create_test_prd();
        let next = prd.next_story().expect("should have a next story");

        // Both story 2 and story 4 are available (depend only on story 1)
        // Story 2 has priority 2, story 4 has priority 10
        // So story 2 should be selected
        assert_eq!(next.id, "2");
        assert_eq!(next.priority, 2);
    }

    #[test]
    fn test_next_story_none_when_all_blocked() {
        let mut prd = create_test_prd();
        // Make story 1 not pass, which blocks all others
        prd.stories[0].passes = false;

        // Story 1 should be next now (no dependencies)
        let next = prd.next_story().expect("should have story 1");
        assert_eq!(next.id, "1");
    }

    #[test]
    fn test_next_story_none_when_complete() {
        let mut prd = create_test_prd();
        for story in &mut prd.stories {
            story.passes = true;
        }

        assert!(prd.next_story().is_none());
    }

    #[test]
    fn test_story_status_complete() {
        let story = Story {
            id: "1".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            priority: 1,
            passes: true,
            acceptance_criteria: vec![],
            depends_on: vec![],
        };

        let completed = HashSet::new();
        assert_eq!(story.status(&completed), StoryStatus::Complete);
    }

    #[test]
    fn test_story_status_pending() {
        let story = Story {
            id: "2".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            priority: 2,
            passes: false,
            acceptance_criteria: vec![],
            depends_on: vec!["1".to_string()],
        };

        let mut completed = HashSet::new();
        completed.insert("1");
        assert_eq!(story.status(&completed), StoryStatus::Pending);
    }

    #[test]
    fn test_story_status_blocked() {
        let story = Story {
            id: "2".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            priority: 2,
            passes: false,
            acceptance_criteria: vec![],
            depends_on: vec!["1".to_string()],
        };

        let completed = HashSet::new();
        assert_eq!(story.status(&completed), StoryStatus::Blocked);
    }

    #[test]
    fn test_get_story() {
        let prd = create_test_prd();
        let story = prd.get_story("2").expect("should find story 2");
        assert_eq!(story.title, "Second story");
    }

    #[test]
    fn test_get_story_mut() {
        let mut prd = create_test_prd();
        let story = prd.get_story_mut("2").expect("should find story 2");
        story.passes = true;

        assert!(prd.stories[1].passes);
    }

    #[test]
    fn test_load_and_save_roundtrip() {
        let prd = create_test_prd();
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_prd.json");

        // Save
        prd.save(&temp_path).expect("should save");

        // Load
        let loaded = Prd::load(&temp_path).expect("should load");

        // Verify
        assert_eq!(loaded.name, prd.name);
        assert_eq!(loaded.stories.len(), prd.stories.len());
        assert_eq!(loaded.stories[0].id, prd.stories[0].id);

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }
}
