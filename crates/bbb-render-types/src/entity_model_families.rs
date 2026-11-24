//! Model families shared by renderer facts and native presentation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndeadHorseModelFamily {
    Skeleton,
    Zombie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaModelFamily {
    Llama,
    TraderLlama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiglinModelFamily {
    Piglin,
    PiglinBrute,
    ZombifiedPiglin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatModelFamily {
    Acacia,
    Bamboo,
    Birch,
    Cherry,
    DarkOak,
    Jungle,
    Mangrove,
    Oak,
    PaleOak,
    Spruce,
}
