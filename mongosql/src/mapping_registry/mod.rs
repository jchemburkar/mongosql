pub use mongosql_datastructures::binding_tuple::Key;
use std::collections::BTreeMap;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MqlReferenceType {
    FieldRef,
    Variable,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MqlMappingRegistryValue {
    pub name: String,
    pub ref_type: MqlReferenceType,
}

impl MqlMappingRegistryValue {
    pub fn new(name: String, ref_type: MqlReferenceType) -> Self {
        MqlMappingRegistryValue { name, ref_type }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MqlMappingRegistry(BTreeMap<Key, MqlMappingRegistryValue>);

impl MqlMappingRegistry {
    pub fn new() -> Self {
        MqlMappingRegistry(BTreeMap::new())
    }

    pub fn with_registry(tree: BTreeMap<Key, MqlMappingRegistryValue>) -> Self {
        MqlMappingRegistry(tree)
    }

    pub fn contains_mapping(&self, scope_level: u16, mapped_k: &String) -> bool {
        self.0
            .iter()
            .any(|(k, v)| k.scope == scope_level && v.name == *mapped_k)
    }

    pub fn get(&self, k: &Key) -> Option<&MqlMappingRegistryValue> {
        self.0.get(k)
    }

    pub fn get_registry(&self) -> &BTreeMap<Key, MqlMappingRegistryValue> {
        &self.0
    }

    pub fn insert<K: Into<Key>, V: Into<MqlMappingRegistryValue>>(
        &mut self,
        k: K,
        v: V,
    ) -> Option<MqlMappingRegistryValue> {
        self.0.insert(k.into(), v.into())
    }

    pub fn merge(&mut self, other: MqlMappingRegistry) -> &mut Self {
        self.0.extend(other.0);
        self
    }
}

impl Default for MqlMappingRegistry {
    fn default() -> Self {
        Self::new()
    }
}
