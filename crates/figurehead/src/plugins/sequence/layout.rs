//! Sequence diagram layout algorithm
//!
//! Calculates positions for participants and messages.

use anyhow::Result;
use unicode_width::UnicodeWidthStr;

use super::database::{Participant, SequenceDatabase};

/// Positioned participant for rendering
#[derive(Debug, Clone)]
pub struct PositionedParticipant {
    pub id: String,
    pub label: String,
    pub x: usize,     // Center x position
    pub width: usize, // Width of the participant box
}

/// Positioned message for rendering
#[derive(Debug, Clone)]
pub struct PositionedMessage {
    pub from_x: usize,
    pub to_x: usize,
    pub y: usize,
    pub label: String,
    pub arrow: super::database::ArrowType,
    pub depth: usize,
}

/// Layout result containing all positioned elements
#[derive(Debug)]
pub struct SequenceLayoutResult {
    pub participants: Vec<PositionedParticipant>,
    pub messages: Vec<PositionedMessage>,
    pub width: usize,
    pub height: usize,
    pub lifeline_start_y: usize, // Y where lifelines begin (after headers)
}

/// Sequence diagram layout algorithm
pub struct SequenceLayoutAlgorithm {
    participant_padding: usize,
    participant_spacing: usize,
    message_height: usize,
    header_height: usize,
}

impl SequenceLayoutAlgorithm {
    pub fn new() -> Self {
        Self {
            participant_padding: 2, // Padding inside participant box
            participant_spacing: 4, // Space between participants
            message_height: 2,      // Vertical space per message
            header_height: 3,       // Space for participant header
        }
    }

    /// Calculate the width needed for a participant
    fn participant_width(&self, participant: &Participant) -> usize {
        let label_width = UnicodeWidthStr::width(participant.label.as_str());
        label_width + self.participant_padding * 2
    }

    /// Layout the diagram
    pub fn layout(&self, database: &SequenceDatabase) -> Result<SequenceLayoutResult> {
        let participants = database.participants();
        let messages: Vec<_> = database.messages().collect();

        if participants.is_empty() {
            return Ok(SequenceLayoutResult {
                participants: Vec::new(),
                messages: Vec::new(),
                width: 0,
                height: 0,
                lifeline_start_y: 0,
            });
        }

        // Calculate participant widths
        let widths: Vec<usize> = participants
            .iter()
            .map(|p| self.participant_width(p))
            .collect();

        // Also consider message label widths that span between participants
        let mut adjusted_spacing = vec![self.participant_spacing; participants.len()];
        // Extra horizontal space needed past the rightmost participant to host
        // self-message labels drawn on the last lifeline.
        let mut right_margin_extra = 0usize;
        // Layout overhead reserved for the self-message marker (e.g. " ↩ ").
        let self_message_overhead = 3usize;
        for msg in &messages {
            if let (Some(from_idx), Some(to_idx)) = (
                database.participant_index(&msg.from),
                database.participant_index(&msg.to),
            ) {
                // Self-message (X->>X) does not span between two participants.
                // The renderer paints it as a loop marker plus trailing label
                // on the participant's lifeline (see draw_self_message). Make
                // sure the canvas has room for the label without colliding
                // with the next lifeline or the right margin.
                if from_idx == to_idx {
                    let label_extra =
                        UnicodeWidthStr::width(msg.label.as_str()) + self_message_overhead;
                    if from_idx + 1 < participants.len() {
                        adjusted_spacing[from_idx] = adjusted_spacing[from_idx]
                            .max(self.participant_spacing + label_extra);
                    } else {
                        right_margin_extra = right_margin_extra.max(label_extra);
                    }
                    continue;
                }

                let (left_idx, right_idx) = if from_idx < to_idx {
                    (from_idx, to_idx)
                } else {
                    (to_idx, from_idx)
                };

                // Message spans from left to right participant
                let label_width = UnicodeWidthStr::width(msg.label.as_str()) + 4; // Arrow chars

                // Calculate current span
                let mut current_span = widths[left_idx] / 2 + widths[right_idx] / 2;
                current_span += adjusted_spacing[left_idx..right_idx].iter().sum::<usize>();
                current_span += widths[(left_idx + 1)..right_idx].iter().sum::<usize>();

                // If label is wider, increase spacing
                if label_width > current_span {
                    let extra = label_width - current_span;
                    // Distribute extra space
                    let slots = right_idx - left_idx;
                    let per_slot = extra.div_ceil(slots);
                    for spacing in &mut adjusted_spacing[left_idx..right_idx] {
                        *spacing = (*spacing).max(self.participant_spacing + per_slot);
                    }
                }
            }
        }

        // Position participants
        let mut positioned_participants = Vec::new();
        let mut x = 2; // Left margin

        for (i, participant) in participants.iter().enumerate() {
            let width = widths[i];
            let center_x = x + width / 2;

            positioned_participants.push(PositionedParticipant {
                id: participant.id.clone(),
                label: participant.label.clone(),
                x: center_x,
                width,
            });

            x += width
                + if i < participants.len() - 1 {
                    adjusted_spacing[i]
                } else {
                    0
                };
        }

        let total_width = x + 2 + right_margin_extra; // Right margin (+ slack for trailing self-message labels)

        // Position messages
        let mut positioned_messages = Vec::new();
        let mut y = self.header_height;

        for msg in &messages {
            if let (Some(from_idx), Some(to_idx)) = (
                database.participant_index(&msg.from),
                database.participant_index(&msg.to),
            ) {
                let from_x = positioned_participants[from_idx].x;
                let to_x = positioned_participants[to_idx].x;

                positioned_messages.push(PositionedMessage {
                    from_x,
                    to_x,
                    y,
                    label: msg.label.clone(),
                    arrow: msg.arrow,
                    depth: msg.depth,
                });

                y += self.message_height;
            }
        }

        // Add space for lifelines after last message
        let total_height = y + 1;

        Ok(SequenceLayoutResult {
            participants: positioned_participants,
            messages: positioned_messages,
            width: total_width,
            height: total_height,
            lifeline_start_y: self.header_height - 1,
        })
    }
}

impl Default for SequenceLayoutAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::database::Message;
    use super::*;

    #[test]
    fn test_empty_layout() {
        let db = SequenceDatabase::new();
        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        assert_eq!(result.participants.len(), 0);
        assert_eq!(result.messages.len(), 0);
    }

    #[test]
    fn test_two_participants() {
        let mut db = SequenceDatabase::new();
        db.add_participant(Participant::new("Alice")).unwrap();
        db.add_participant(Participant::new("Bob")).unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        assert_eq!(result.participants.len(), 2);
        assert!(result.participants[0].x < result.participants[1].x);
    }

    #[test]
    fn test_message_positioning() {
        let mut db = SequenceDatabase::new();
        db.add_message(Message::new("Alice", "Bob", "Hello"))
            .unwrap();
        db.add_message(Message::new("Bob", "Alice", "Hi")).unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        assert_eq!(result.messages.len(), 2);
        // Second message should be below first
        assert!(result.messages[1].y > result.messages[0].y);
    }

    #[test]
    fn test_message_direction() {
        let mut db = SequenceDatabase::new();
        db.add_message(Message::new("Alice", "Bob", "Right"))
            .unwrap();
        db.add_message(Message::new("Bob", "Alice", "Left"))
            .unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        // First message goes right (from_x < to_x)
        assert!(result.messages[0].from_x < result.messages[0].to_x);
        // Second message goes left (from_x > to_x)
        assert!(result.messages[1].from_x > result.messages[1].to_x);
    }

    #[test]
    fn self_message_does_not_panic() {
        let mut db = SequenceDatabase::new();
        db.add_message(Message::new("Alice", "Alice", "introspect"))
            .unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout
            .layout(&db)
            .expect("self-message layout must not panic");

        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].from_x, result.messages[0].to_x);
    }

    #[test]
    fn self_message_on_last_participant_extends_right_margin() {
        let mut db = SequenceDatabase::new();
        db.add_participant(Participant::new("Alice")).unwrap();
        db.add_participant(Participant::new("Bob")).unwrap();
        db.add_message(Message::new(
            "Bob",
            "Bob",
            "very_long_self_message_label_indeed",
        ))
        .unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        // Total width must accommodate the trailing self-message label.
        let last_x = result.participants.last().unwrap().x;
        assert!(
            result.width > last_x + "very_long_self_message_label_indeed".len(),
            "canvas width {} should leave room past last lifeline at {} for the label",
            result.width,
            last_x
        );
    }

    #[test]
    fn self_message_between_participants_widens_neighbor_spacing() {
        let mut db = SequenceDatabase::new();
        db.add_participant(Participant::new("Alice")).unwrap();
        db.add_participant(Participant::new("Bob")).unwrap();
        db.add_message(Message::new(
            "Alice",
            "Alice",
            "very_long_self_message_label_indeed",
        ))
        .unwrap();

        let layout = SequenceLayoutAlgorithm::new();
        let result = layout.layout(&db).unwrap();

        let alice_x = result.participants[0].x;
        let bob_x = result.participants[1].x;
        assert!(
            bob_x - alice_x > "very_long_self_message_label_indeed".len(),
            "neighbor spacing {} must exceed self-message label width to avoid lifeline collision",
            bob_x - alice_x
        );
    }
}
