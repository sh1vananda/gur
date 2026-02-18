//! Dialogue tree — loaded from RON, driven by the narrative system.

use serde::{Deserialize, Serialize};

/// An action triggered when a dialogue node completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueAction {
    /// Open the shop interface.
    OpenShop,
    /// Set a world flag.
    SetFlag { key: String, value: bool },
    /// Give the player an item.
    GiveItem { item_id: String, count: u32 },
    /// Start a quest.
    StartQuest { quest_id: String },
    /// Transition to a zone.
    ZoneTransition { zone_id: String },
    /// No action.
    None,
}

/// A player choice presented at a dialogue decision point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Text shown to the player.
    pub text: String,
    /// ID of the next node to show, or `None` to end dialogue.
    pub next: Option<String>,
    /// World flag required for this choice to appear. `None` = always visible.
    pub requires_flag: Option<String>,
}

/// A single node in the dialogue tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    /// Unique node identifier.
    pub id: String,
    /// The text spoken by the NPC (or narration).
    pub text: String,
    /// Speaker name shown in the dialogue box. `None` = no label.
    pub speaker: Option<String>,
    /// Choices presented to the player. Empty = auto-advance.
    pub choices: Vec<DialogueChoice>,
    /// Action to execute when this node is reached.
    pub action: DialogueAction,
    /// If `Some`, advance to this node automatically (no choices shown).
    pub auto_next: Option<String>,
}

/// A complete dialogue tree for an NPC or scene.
///
/// # RON format:
/// ```ron
/// DialogueTree(
///     id: "merchant_main",
///     entry: "start",
///     nodes: [
///         Node(id: "start", text: "Ah, a traveller...", choices: [
///             Choice(text: "What do you sell?", next: Some("shop")),
///             Choice(text: "Goodbye.", next: None),
///         ], action: None, auto_next: None, speaker: Some("Merchant")),
///     ]
/// )
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTree {
    /// Unique ID for this dialogue tree.
    pub id: String,
    /// ID of the first node to show.
    pub entry: String,
    /// All nodes in this tree.
    pub nodes: Vec<DialogueNode>,
}

impl DialogueTree {
    /// Find a node by ID.
    pub fn node(&self, id: &str) -> Option<&DialogueNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get the entry node.
    pub fn entry_node(&self) -> Option<&DialogueNode> {
        self.node(&self.entry)
    }

    /// Load a dialogue tree from a RON string.
    pub fn from_ron(src: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(src)
    }
}
