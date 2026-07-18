use serde::Serialize;

use crate::model::{ForkJoinPhase, Node, RuleDocument, Schedule, TriggerNode};

#[derive(Serialize)]
struct SemanticProjection<'a> {
    schema: &'a crate::model::SchemaLock,
    semantic: crate::model::SemanticRule,
}

pub(crate) fn canonical_semantic_bytes(document: &RuleDocument) -> serde_json::Result<Vec<u8>> {
    let mut semantic = document.semantic.clone();
    semantic
        .nodes
        .sort_by(|left, right| left.id().cmp(right.id()));
    semantic.edges.sort_by(|left, right| left.id.cmp(&right.id));
    if let TriggerNode::TimeSchedule {
        schedule: Schedule::WeeklyCivil { days, .. },
        ..
    } = &mut semantic.trigger
    {
        days.sort();
    }
    for node in &mut semantic.nodes {
        match node {
            Node::Condition { expression, .. } => expression
                .inputs
                .sort_by(|left, right| left.name.cmp(&right.name)),
            Node::ForkJoin { phase, .. } => {
                let branches = match phase {
                    ForkJoinPhase::Fork { branches, .. } | ForkJoinPhase::Join { branches, .. } => {
                        branches
                    }
                };
                branches.sort_by(|left, right| {
                    left.order
                        .cmp(&right.order)
                        .then_with(|| left.id.cmp(&right.id))
                });
            }
            _ => {}
        }
    }
    serde_json_canonicalizer::to_vec(&SemanticProjection {
        schema: &document.schema,
        semantic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures::fixture_values, schema::artifact_digests};

    #[test]
    fn fork_branch_source_order_is_not_semantic() {
        let value = fixture_values(&artifact_digests())
            .into_iter()
            .find(|fixture| fixture.path == "fixtures/valid-all-node-families.json")
            .unwrap()
            .value;
        let document: RuleDocument = serde_json::from_value(value).unwrap();
        let original = canonical_semantic_bytes(&document).unwrap();
        let mut reordered = document.clone();
        for node in &mut reordered.semantic.nodes {
            if let Node::ForkJoin { phase, .. } = node {
                match phase {
                    ForkJoinPhase::Fork { branches, .. } | ForkJoinPhase::Join { branches, .. } => {
                        branches.reverse()
                    }
                }
            }
        }
        assert_eq!(original, canonical_semantic_bytes(&reordered).unwrap());
    }
}
