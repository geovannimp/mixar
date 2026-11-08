use crate::refs::validate_id;
use crate::{presets, CurveError, CurveGroup, CurveGroupRef, CurveRefKind};

/// Maximum custom groups a [`CurveLibrary`] may hold.
pub const MAX_CUSTOM_GROUPS: usize = 64;
/// Maximum curve sets a [`CurveLibrary`] may hold.
pub const MAX_SETS: usize = 32;
/// Maximum members in one [`CurveSet`].
pub const MAX_SET_MEMBERS: usize = 32;

/// A curve group with a stable snake_case id and a UI-only display name.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedCurveGroup {
    pub id: String,
    pub display_name: String,
    pub group: CurveGroup,
}

impl NamedCurveGroup {
    /// Validate and construct a named group.
    ///
    /// An empty `display_name` falls back to `id`.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        group: CurveGroup,
    ) -> Result<Self, CurveError> {
        let id = id.into();
        let display_name = display_name.into();
        let named = Self {
            display_name: if display_name.is_empty() {
                id.clone()
            } else {
                display_name
            },
            id,
            group,
        };
        named.validate()?;
        Ok(named)
    }

    /// Ids are canonical snake_case and may not shadow a built-in preset.
    pub fn validate(&self) -> Result<(), CurveError> {
        validate_id(&self.id)?;
        if presets::is_preset_id(&self.id) {
            return Err(CurveError::ReservedId {
                id: self.id.clone(),
            });
        }
        self.group.validate()
    }

    /// A reference suitable for a [`CurveSet`] member.
    pub fn group_ref(&self) -> CurveGroupRef {
        CurveGroupRef::custom(self.id.clone())
    }
}

/// An ordered set of curve groups addressed by one normalized knob position.
///
/// A knob position sweeps the members and blends adjacent groups; the layer
/// that owns the target performs that interpolation. Curve-core only validates
/// that the set is non-empty, within the member limit, and well-formed.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveSet {
    pub id: String,
    pub display_name: String,
    pub groups: Vec<CurveGroupRef>,
}

impl CurveSet {
    /// Validate and construct a curve set.
    ///
    /// An empty `display_name` falls back to `id`.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        groups: Vec<CurveGroupRef>,
    ) -> Result<Self, CurveError> {
        let id = id.into();
        let display_name = display_name.into();
        let set = Self {
            display_name: if display_name.is_empty() {
                id.clone()
            } else {
                display_name
            },
            id,
            groups,
        };
        set.validate()?;
        Ok(set)
    }

    /// Validate the id, member count, and every member reference.
    pub fn validate(&self) -> Result<(), CurveError> {
        validate_id(&self.id)?;
        if self.groups.is_empty() {
            return Err(CurveError::EmptySet {
                id: self.id.clone(),
            });
        }
        if self.groups.len() > MAX_SET_MEMBERS {
            return Err(CurveError::TooManySetMembers {
                id: self.id.clone(),
                count: self.groups.len(),
            });
        }
        for member in &self.groups {
            member.validate()?;
        }
        Ok(())
    }
}

/// Named custom curve groups plus ordered curve sets.
///
/// Presets are not stored here: ids in [`CurveRefKind::Preset`] are resolved by
/// the layer that owns the preset registry. Custom references must resolve to a
/// group in this library.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveLibrary {
    pub groups: Vec<NamedCurveGroup>,
    pub sets: Vec<CurveSet>,
}

impl CurveLibrary {
    /// Validate and construct a library from its parts.
    pub fn new(groups: Vec<NamedCurveGroup>, sets: Vec<CurveSet>) -> Result<Self, CurveError> {
        let library = Self { groups, sets };
        library.validate()?;
        Ok(library)
    }

    /// Validate limits, ids, groups, set members, and custom references.
    pub fn validate(&self) -> Result<(), CurveError> {
        if self.groups.len() > MAX_CUSTOM_GROUPS {
            return Err(CurveError::TooManyGroups {
                count: self.groups.len(),
            });
        }
        if self.sets.len() > MAX_SETS {
            return Err(CurveError::TooManySets {
                count: self.sets.len(),
            });
        }

        for (index, group) in self.groups.iter().enumerate() {
            group.validate().map_err(|source| CurveError::NamedGroup {
                id: group.id.clone(),
                source: Box::new(source),
            })?;
            if self.groups[..index]
                .iter()
                .any(|other| other.id == group.id)
            {
                return Err(CurveError::DuplicateGroupId {
                    id: group.id.clone(),
                });
            }
        }

        for (index, set) in self.sets.iter().enumerate() {
            set.validate().map_err(|source| CurveError::NamedSet {
                id: set.id.clone(),
                source: Box::new(source),
            })?;
            if self.sets[..index].iter().any(|other| other.id == set.id) {
                return Err(CurveError::DuplicateSetId { id: set.id.clone() });
            }
            for member in &set.groups {
                if let CurveRefKind::Custom { id } = &member.kind {
                    if !self.groups.iter().any(|group| group.id == *id) {
                        return Err(CurveError::UnknownGroup {
                            id: set.id.clone(),
                            group_id: id.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Insert a group, enforcing the limit, id rules, and uniqueness.
    pub fn insert_group(&mut self, group: NamedCurveGroup) -> Result<(), CurveError> {
        if self.groups.len() >= MAX_CUSTOM_GROUPS {
            return Err(CurveError::TooManyGroups {
                count: self.groups.len() + 1,
            });
        }
        group.validate().map_err(|source| CurveError::NamedGroup {
            id: group.id.clone(),
            source: Box::new(source),
        })?;
        if self.groups.iter().any(|other| other.id == group.id) {
            return Err(CurveError::DuplicateGroupId { id: group.id });
        }
        self.groups.push(group);
        Ok(())
    }

    /// Insert a set, enforcing the limit, id rules, and custom references.
    pub fn insert_set(&mut self, set: CurveSet) -> Result<(), CurveError> {
        if self.sets.len() >= MAX_SETS {
            return Err(CurveError::TooManySets {
                count: self.sets.len() + 1,
            });
        }
        set.validate().map_err(|source| CurveError::NamedSet {
            id: set.id.clone(),
            source: Box::new(source),
        })?;
        if self.sets.iter().any(|other| other.id == set.id) {
            return Err(CurveError::DuplicateSetId { id: set.id });
        }
        for member in &set.groups {
            if let CurveRefKind::Custom { id } = &member.kind {
                if !self.groups.iter().any(|group| group.id == *id) {
                    return Err(CurveError::UnknownGroup {
                        id: set.id.clone(),
                        group_id: id.clone(),
                    });
                }
            }
        }
        self.sets.push(set);
        Ok(())
    }

    /// A custom group by id.
    pub fn group(&self, id: &str) -> Option<&NamedCurveGroup> {
        self.groups.iter().find(|group| group.id == id)
    }

    /// A curve set by id.
    pub fn set(&self, id: &str) -> Option<&CurveSet> {
        self.sets.iter().find(|set| set.id == id)
    }

    /// Resolve a custom reference; preset references return `None` because the
    /// preset registry lives above this crate.
    pub fn resolve_group(&self, reference: &CurveGroupRef) -> Option<&CurveGroup> {
        match &reference.kind {
            CurveRefKind::Custom { id } => self.group(id).map(|group| &group.group),
            CurveRefKind::Preset { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CurvePoint, CurveSpec, SegmentKind};

    fn linear_group() -> CurveGroup {
        let curve = CurveSpec::new(
            vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)],
            vec![SegmentKind::Linear],
        )
        .expect("valid curve");
        CurveGroup::scalar(curve).expect("valid group")
    }

    fn named(id: &str) -> NamedCurveGroup {
        NamedCurveGroup::new(id, "", linear_group()).expect("valid named group")
    }

    #[test]
    fn named_group_defaults_display_name_and_exposes_a_ref() {
        let group = named("my_curve");
        assert_eq!(group.display_name, "my_curve");
        assert_eq!(group.group_ref(), CurveGroupRef::custom("my_curve"));
    }

    #[test]
    fn library_validates_groups_sets_and_custom_references() {
        let group = named("my_curve");
        let reference = group.group_ref();
        let set = CurveSet::new(
            "my_set",
            "My Set",
            vec![CurveGroupRef::preset("linear"), reference],
        )
        .expect("valid set");
        let library = CurveLibrary::new(vec![group], vec![set]).expect("valid library");
        assert!(library.group("my_curve").is_some());
        assert!(library.set("my_set").is_some());
        assert!(library
            .resolve_group(&CurveGroupRef::custom("my_curve"))
            .is_some());
        assert!(library
            .resolve_group(&CurveGroupRef::preset("linear"))
            .is_none());
    }

    #[test]
    fn rejects_duplicate_and_reserved_ids() {
        assert!(matches!(
            CurveLibrary::new(vec![named("dup"), named("dup")], Vec::new()),
            Err(CurveError::DuplicateGroupId { .. })
        ));
        assert!(matches!(
            NamedCurveGroup::new("linear", "", linear_group()),
            Err(CurveError::ReservedId { .. })
        ));
        assert!(matches!(
            NamedCurveGroup::new("Bad-Id", "", linear_group()),
            Err(CurveError::InvalidId { .. })
        ));
    }

    #[test]
    fn rejects_unknown_custom_member() {
        let set = CurveSet::new("my_set", "", vec![CurveGroupRef::custom("missing")])
            .expect("syntactically valid set");
        assert!(matches!(
            CurveLibrary::new(Vec::new(), vec![set]),
            Err(CurveError::UnknownGroup { .. })
        ));
    }

    #[test]
    fn rejects_empty_sets_and_too_many_members() {
        assert!(matches!(
            CurveSet::new("empty", "", Vec::new()),
            Err(CurveError::EmptySet { .. })
        ));
        let members = vec![CurveGroupRef::preset("linear"); MAX_SET_MEMBERS + 1];
        assert!(matches!(
            CurveSet::new("big", "", members),
            Err(CurveError::TooManySetMembers { count: 33, .. })
        ));
    }

    #[test]
    fn enforces_library_limits() {
        let groups = (0..=MAX_CUSTOM_GROUPS)
            .map(|index| named(&format!("curve_{index}")))
            .collect();
        assert!(matches!(
            CurveLibrary::new(groups, Vec::new()),
            Err(CurveError::TooManyGroups { count: 65 })
        ));

        let sets = (0..=MAX_SETS)
            .map(|index| {
                CurveSet::new(
                    format!("set_{index}"),
                    "",
                    vec![CurveGroupRef::preset("linear")],
                )
                .expect("valid set")
            })
            .collect();
        assert!(matches!(
            CurveLibrary::new(Vec::new(), sets),
            Err(CurveError::TooManySets { count: 33 })
        ));
    }

    #[test]
    fn insert_enforces_uniqueness_and_reference_resolution() {
        let mut library = CurveLibrary::default();
        library.insert_group(named("a")).expect("first insert");
        assert!(matches!(
            library.insert_group(named("a")),
            Err(CurveError::DuplicateGroupId { .. })
        ));

        let missing = CurveSet::new("set_a", "", vec![CurveGroupRef::custom("missing")])
            .expect("syntactically valid");
        assert!(matches!(
            library.insert_set(missing),
            Err(CurveError::UnknownGroup { .. })
        ));

        let ok = CurveSet::new("set_a", "", vec![CurveGroupRef::custom("a")]).expect("valid set");
        library.insert_set(ok).expect("resolved insert");
        assert!(library.set("set_a").is_some());
    }
}
