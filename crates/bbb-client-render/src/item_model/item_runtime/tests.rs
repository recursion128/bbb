use super::*;

use bbb_protocol::data_component_types as component_id;

use bbb_protocol::mob_effect_types::{VANILLA_MOB_EFFECT_HASTE_ID, VANILLA_MOB_EFFECT_SLOWNESS_ID};

use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

mod batch_01;
mod batch_02;
mod batch_03;
mod batch_04;
mod batch_05;
mod batch_06;
mod support_01;
mod support_02;
mod support_03;
mod support_04;
#[allow(unused_imports)]
use support_01::*;
use support_02::*;
use support_03::*;
use support_04::*;
