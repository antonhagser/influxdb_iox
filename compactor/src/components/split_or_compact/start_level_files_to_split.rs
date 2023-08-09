use data_types::{CompactionLevel, ParquetFile, Timestamp};
use itertools::Itertools;
use observability_deps::tracing::debug;

use crate::{
    components::files_split::{target_level_split::TargetLevelSplit, FilesSplit},
    file_classification::FileToSplit,
};

// selectSplitTimes returns an appropriate sets of split times to divide the given time range into,
// based on how much over the max_compact_size the capacity is.
// The assumption is that the caller has `cap` bytes spread across `min_time` to `max_time` and wants
// to split those bytes into chunks approximating `max_compact_size`.  We don't know if the data is
// spread linearly, so we'll split into more pieces than would be necesary if the data is linear.
// The cost of splitting into smaller pieces is minimal (potentially extra but smaller compactions),
// while the cost splitting into pieces that are too big is considerable (we may have split again).
// A vec of split_hints can be provided, which is assumed to be the min/max file times of the target
// level files.  When hints are specified, this function will try to split at the hint times, if they're
// +/- 50% the computed split times.
pub fn select_split_times(
    cap: usize,
    max_compact_size: usize,
    min_time: i64,
    max_time: i64,
    split_hint: Vec<i64>,
) -> Vec<i64> {
    if min_time == max_time {
        // can't split below 1 ns.
        return vec![];
    }

    // If the bytes are spread perfectly even across `min_time` to `max_time`, the ideal number of splits
    // would simply be cap / max_compact_size.
    let mut splits = cap / max_compact_size;

    // But the data won't be spread perfectly even across the time range, and its better err towards splitting
    // extra small rather than splitting extra large (which may still exceed max_compact_size).
    // So pad the split count beyond what a perfect distribution would require, by doubling it.
    splits *= 2;

    // Splitting the time range into `splits` pieces requires an increase between each split time of `delta`.
    let mut default_delta = (max_time - min_time) / (splits + 1) as i64;
    let mut min_delta = default_delta / 2; // used to decide how far we'll deviate to align to split_hint
    let mut max_delta = default_delta * 3 / 2; // used to decide how far we'll deviate to align to split_hint

    if default_delta == 0 {
        // The computed count leads to splitting at less than 1ns, which we cannot do.
        splits = (max_time - min_time) as usize; // The maximum number of splits possible for this time range.
        default_delta = 1; // The smallest time delta between splits possible for this time range.
    }
    min_delta = min_delta.max(1);
    max_delta = max_delta.max(1);

    let mut split_time = min_time;
    let mut split_times = Vec::with_capacity(splits);
    let mut hint_idx = 0;

    // The * 3 / 2 in the loop criteria is so that we don't split at the very end of the time range, resulting
    // in an unnecessary tiny time slice at the end.
    while split_time + default_delta * 3 / 2 < max_time {
        // advance to the next possible hint
        while hint_idx < split_hint.len() && split_hint[hint_idx] < split_time + min_delta {
            hint_idx += 1;
        }

        // if there's multiple hints we could use for the next split time, chose the closest one to our default delta.
        let default_next = split_time + default_delta;
        while hint_idx + 1 < split_hint.len()
            && (split_hint[hint_idx] - default_next).abs()
                > (split_hint[hint_idx + 1] - default_next).abs()
        {
            hint_idx += 1;
        }

        split_time = if hint_idx < split_hint.len() && split_hint[hint_idx] < split_time + max_delta
        {
            // The next hint is close enough to the next split that we'll use it instead of the computed split.
            split_hint[hint_idx]
        } else {
            // There is no next hint, or its too far away, so add the default to the last split time.
            split_time + default_delta
        };

        if split_time < max_time {
            split_times.push(split_time);
        }
    }

    split_times
}

// split_into_chains splits files into separate overlapping chains of files.
// A chain is a series of files that overlap.  Each file in the chain overlaps at least 1 neighbor, but all files
// in the chain may not overlap all other files in the chain.  A "chain" is identified by sorting by min_time.
// When the first file overlaps at least the next file.  As long as at least one prior file overlaps the next file,
// the chain continues.  When the next file (by min_time) does not overlap a prior file, the chain ends, and a new
// chain begins.
pub fn split_into_chains(mut files: Vec<ParquetFile>) -> Vec<Vec<ParquetFile>> {
    let mut left = files.len(); // how many files remain to consider
    let mut chains: Vec<Vec<ParquetFile>> = Vec::with_capacity(10);
    let mut chain: Vec<ParquetFile> = Vec::with_capacity(left);
    let mut max_time: Timestamp = Timestamp::new(0);

    files.sort_by_key(|f| f.min_time);

    for file in files.drain(..) {
        if chain.is_empty() {
            // first of new chain
            max_time = file.max_time;
            chain.push(file);
        } else if file.min_time <= max_time {
            // This overlaps the chain, add to it.
            if file.max_time > max_time {
                max_time = file.max_time;
            }
            chain.push(file);
        } else {
            // file does not overlap the chain, its the start of a new chain.
            max_time = file.max_time;
            chains.push(chain);
            chain = Vec::with_capacity(left);
            chain.push(file);
        }
        left -= 1;
    }
    chains.push(chain);
    chains
}

// merge_small_l0_chains takes a vector of overlapping "chains" (where a chain is vector of overlapping L0 files), and
// attempts to merge small chains together if doing so can keep them under the given max_compact_size.
// This function makes no assumption about the order of the chains - if they are created by `split_into_chains`, they're
// ordered by min_time, which is unsafe for merging L0 chains.
pub fn merge_small_l0_chains(
    mut chains: Vec<Vec<ParquetFile>>,
    max_compact_size: usize,
) -> Vec<Vec<ParquetFile>> {
    chains.sort_by_key(|a| get_max_l0_created_at(a.to_vec()));
    let mut merged_chains: Vec<Vec<ParquetFile>> = Vec::with_capacity(chains.len());
    let mut prior_chain_bytes: usize = 0;
    let mut prior_chain_idx: i32 = -1;
    for chain in &chains {
        let this_chain_bytes = chain.iter().map(|f| f.file_size_bytes as usize).sum();

        // matching max_lo_created_at times indicates that the files were deliberately split.  We shouldn't merge
        // chains with matching max_lo_created_at times, because that would encourage undoing the previous split,
        // which minimally increases write amplification, and may cause unproductive split/compact loops.
        let mut matches = 0;
        if prior_chain_bytes > 0 {
            for f in chain.iter() {
                for f2 in &merged_chains[prior_chain_idx as usize] {
                    if f.max_l0_created_at == f2.max_l0_created_at {
                        matches += 1;
                        break;
                    }
                }
            }
        }

        // Merge it if: there a prior chain to merge with, and merging wouldn't make it too big, or undo a previous split
        if prior_chain_bytes > 0
            && prior_chain_bytes + this_chain_bytes <= max_compact_size
            && matches == 0
        {
            // this chain can be added to the prior chain.
            merged_chains[prior_chain_idx as usize].append(&mut chain.clone());
            prior_chain_bytes += this_chain_bytes;
        } else {
            merged_chains.push(chain.to_vec());
            prior_chain_bytes = this_chain_bytes;
            prior_chain_idx += 1;
        }
    }

    merged_chains
}

// get_max_l0_created_at gets the highest max_l0_created_at from all files within a vec.
fn get_max_l0_created_at(files: Vec<ParquetFile>) -> Timestamp {
    files
        .into_iter()
        .map(|f| f.max_l0_created_at)
        .max()
        .unwrap()
}

/// Return (`[files_to_split]`, `[files_not_to_split]`) of given files
/// such that `files_to_split` are files  in start-level that overlaps with more than one file in target_level.
///
/// The returned `[files_to_split]` includes a set of pairs. A pair is composed of a file and its corresponding split-times
/// at which the file will be split into multiple files.
///
/// Unlike high_l0_overlap_split, this function focusses on scenarios where start level files are not highly overlapping.
/// Typically each file overlaps its neighbors, but each file does not overlap all or almost all L0.s
///
/// Example:
///  . Input:
///                          |---L0.1---|   |--L0.2--|
///            |--L1.1--| |--L1.2--| |--L1.3--|
///
///    L0.1 overlaps with 2 level-1 files (L1.2, L1.3) and should be split into 2 files, one overlaps with L1.2
///    and one oerlaps with L1.3
///
///  . Output:
///     . files_to_split = [L0.1]
///     . files_not_to_split = [L1.1, L1.2, L1.3, L0.2] which is the rest of the files
///
/// Reason behind the split:
/// Since a start-level file needs to compact with all of its overlapped target-level files to retain the invariant that
/// all files in target level are non-overlapped, splitting start-level files is to reduce the number of overlapped files
/// at the target level and avoid compacting too many files in the next compaction cycle.
/// To achieve this goal, a start-level file should be split to overlap with at most one target-level file. This enables the
/// minimum set of compacting files to 2 files: a start-level file and an overlapped target-level file.
///
pub fn identify_start_level_files_to_split(
    files: Vec<ParquetFile>,
    target_level: CompactionLevel,
) -> (Vec<FileToSplit>, Vec<ParquetFile>) {
    // panic if not all files are either in target level or start level
    let start_level = target_level.prev();
    assert!(files
        .iter()
        .all(|f| f.compaction_level == target_level || f.compaction_level == start_level));

    // Get start-level and target-level files
    let len = files.len();
    let split = TargetLevelSplit::new();
    let (mut start_level_files, mut target_level_files) = split.apply(files, start_level);

    // sort start_level files in their max_l0_created_at
    start_level_files.sort_by_key(|f| f.max_l0_created_at);
    // sort target level files in their min_time
    target_level_files.sort_by_key(|f| f.min_time);

    // Get files in start level that overlap with any file in target level
    let mut files_to_split = Vec::with_capacity(len);
    let mut files_not_to_split = Vec::with_capacity(len);
    for file in start_level_files {
        // Get target_level files that overlaps with this file
        let overlapped_target_level_files: Vec<&ParquetFile> = target_level_files
            .iter()
            .filter(|f| file.overlaps(f))
            .collect();

        // Neither split file that overlaps with only one file in target level
        // nor has a single timestamp (splitting this will lead to the same file and as a result will introduce infinite loop)
        // nor has time range = 1 (splitting this will cause panic because split_time will be min_tim/max_time which is disallowed)
        if overlapped_target_level_files.len() < 2 || file.min_time == file.max_time {
            files_not_to_split.push(file);
        } else {
            debug!(?file.min_time, ?file.max_time, ?file.compaction_level, "time range of file to split");
            overlapped_target_level_files
                .iter()
                .for_each(|f| debug!(?f.min_time, ?f.max_time, ?f.compaction_level, "time range of overlap file"));

            // this files will be split, add its max time
            let split_times: Vec<i64> = overlapped_target_level_files
                .iter()
                .filter(|f| f.max_time < file.max_time)
                .map(|f| f.max_time.get())
                .dedup()
                .collect();

            debug!(?split_times);

            files_to_split.push(FileToSplit { file, split_times });
        }
    }

    // keep the rest of the files for next round
    files_not_to_split.extend(target_level_files);

    assert_eq!(files_to_split.len() + files_not_to_split.len(), len);

    (files_to_split, files_not_to_split)
}

#[cfg(test)]
mod tests {
    use compactor_test_utils::{
        create_l1_files, create_overlapped_files, create_overlapped_l0_l1_files_2, format_files,
        format_files_split,
    };
    use data_types::CompactionLevel;

    #[test]
    fn test_select_split_times() {
        // First some normal cases:

        // splitting 150 bytes based on a max of 100, with a time range 0-100, gives 2 splits, into 3 pieces.
        // 1 split into 2 pieces would have also been ok.
        let mut split_times = super::select_split_times(150, 100, 0, 100, vec![]);
        assert!(split_times == vec![33, 66]);
        // give it hints (overlapping L1s) that are close to the splits it choses by default, and it will use them.
        split_times = super::select_split_times(150, 100, 0, 100, vec![30, 65]);
        assert!(split_times == vec![30, 65]);
        // give it hints (overlapping L1s) that are far the splits it choses by default, and it sticks with the default.
        split_times = super::select_split_times(150, 100, 0, 100, vec![10, 95]);
        assert!(split_times == vec![33, 66]);

        // splitting 199 bytes based on a max of 100, with a time range 0-100, gives 2 splits, into 3 pieces.
        // 1 split into 2 pieces would be a bad choice - its any deviation from perfectly linear distribution
        // would cause the split range to still exceed the max.
        split_times = super::select_split_times(199, 100, 0, 100, vec![]);
        assert!(split_times == vec![33, 66]);

        // splitting 200-299 bytes based on a max of 100, with a time range 0-100, gives 4 splits into 5 pieces.
        // A bit agressive for exactly 2x the max cap, but very reasonable for 1 byte under 3x.
        split_times = super::select_split_times(200, 100, 0, 100, vec![]);
        assert!(split_times == vec![20, 40, 60, 80]);
        split_times = super::select_split_times(299, 100, 0, 100, vec![]);
        assert!(split_times == vec![20, 40, 60, 80]);
        // once a hint shifts the split times, the rest of the split times are shifted too.
        split_times = super::select_split_times(299, 100, 0, 100, vec![43]);
        assert!(split_times == vec![20, 43, 63, 83]);
        // give it a lot of hints, and see it pick the best (closest) ones.
        split_times =
            super::select_split_times(299, 100, 0, 100, vec![15, 19, 23, 35, 41, 55, 61, 82, 83]);
        assert!(split_times == vec![19, 41, 61, 82]);

        // splitting 300-399 bytes based on a max of 100, with a time range 0-100, gives 5 splits, 6 pieces.
        // A bit agressive for exactly 3x the max cap, but very reasonable for 1 byte under 4x.
        split_times = super::select_split_times(300, 100, 0, 100, vec![]);
        assert!(split_times == vec![14, 28, 42, 56, 70, 84]);
        split_times = super::select_split_times(399, 100, 0, 100, vec![]);
        assert!(split_times == vec![14, 28, 42, 56, 70, 84]);

        // splitting 400 bytes based on a max of 100, with a time range 0-100, gives 7 splits, 8 pieces.
        split_times = super::select_split_times(400, 100, 0, 100, vec![]);
        assert!(split_times == vec![11, 22, 33, 44, 55, 66, 77, 88]);

        // Now some pathelogical cases:

        // splitting 400 bytes based on a max of 100, with a time range 0-3, gives 2 splits, into 3 pieces.
        // Some (maybe all) of these will still exceed the max, but this is the most splitting possible for
        // the time range.
        split_times = super::select_split_times(400, 100, 0, 3, vec![]);
        assert!(split_times == vec![1, 2]);
    }

    #[test]
    fn test_split_empty() {
        let files = vec![];
        let (files_to_split, files_not_to_split) =
            super::identify_start_level_files_to_split(files, CompactionLevel::Initial);
        assert!(files_to_split.is_empty());
        assert!(files_not_to_split.is_empty());
    }

    #[test]
    #[should_panic]
    fn test_split_files_wrong_target_level() {
        // all L1 files
        let files = create_l1_files(1);

        // Target is L0 while all files are in L1 --> panic
        let (_files_to_split, _files_not_to_split) =
            super::identify_start_level_files_to_split(files, CompactionLevel::Initial);
    }

    #[test]
    #[should_panic]
    fn test_split_files_three_level_files() {
        // Three level files
        let files = create_overlapped_files();
        insta::assert_yaml_snapshot!(
            format_files("initial", &files),
            @r###"
        ---
        - initial
        - "L0                                                                                                                 "
        - "L0.2[650,750] 0ns 1b                                                                      |--L0.2--|               "
        - "L0.1[450,620] 0ns 1b                                                  |-----L0.1------|                            "
        - "L0.3[800,900] 0ns 100b                                                                                   |--L0.3--|"
        - "L1                                                                                                                 "
        - "L1.13[600,700] 0ns 100b                                                              |-L1.13--|                    "
        - "L1.12[400,500] 0ns 1b                                            |-L1.12--|                                        "
        - "L1.11[250,350] 0ns 1b                             |-L1.11--|                                                       "
        - "L2                                                                                                                 "
        - "L2.21[0,100] 0ns 1b      |-L2.21--|                                                                                "
        - "L2.22[200,300] 0ns 1b                        |-L2.22--|                                                            "
        "###
        );

        // panic because it only handle at most 2 levels next to each other
        let (_files_to_split, _files_not_to_split) =
            super::identify_start_level_files_to_split(files, CompactionLevel::FileNonOverlapped);
    }

    #[test]
    fn test_split_files_no_split() {
        let files = create_l1_files(1);
        insta::assert_yaml_snapshot!(
            format_files("initial", &files),
            @r###"
        ---
        - initial
        - "L1, all files 1b                                                                                                   "
        - "L1.13[600,700] 0ns                                                                             |------L1.13-------|"
        - "L1.12[400,500] 0ns                                     |------L1.12-------|                                        "
        - "L1.11[250,350] 0ns       |------L1.11-------|                                                                      "
        "###
        );

        let (files_to_split, files_not_to_split) =
            super::identify_start_level_files_to_split(files, CompactionLevel::FileNonOverlapped);
        assert!(files_to_split.is_empty());
        assert_eq!(files_not_to_split.len(), 3);
    }

    #[test]
    fn test_split_files_split() {
        let files = create_overlapped_l0_l1_files_2(1);
        insta::assert_yaml_snapshot!(
            format_files("initial", &files),
            @r###"
        ---
        - initial
        - "L0, all files 1b                                                                                                   "
        - "L0.2[650,750] 180s                                                    |------L0.2------|                           "
        - "L0.1[450,620] 120s                |------------L0.1------------|                                                   "
        - "L0.3[800,900] 300s                                                                               |------L0.3------|"
        - "L1, all files 1b                                                                                                   "
        - "L1.13[600,700] 60s                                           |-----L1.13------|                                    "
        - "L1.12[400,500] 60s       |-----L1.12------|                                                                        "
        "###
        );

        let (files_to_split, files_not_to_split) =
            super::identify_start_level_files_to_split(files, CompactionLevel::FileNonOverlapped);

        // L0.1 that overlaps with 2 level-1 files will be split
        assert_eq!(files_to_split.len(), 1);

        // L0.1 [450, 620] will be split at 500 (max of its overlapped L1.12)
        // The spit_times [500] means after we execute the split (in later steps), L0.1 will
        // be split into 2 files with time ranges: [450, 500] and [501, 620]. This means the first file will
        // overlap with L1.12 and the second file will overlap with L1.13
        assert_eq!(files_to_split[0].file.id.get(), 1);
        assert_eq!(files_to_split[0].split_times, vec![500]);

        // The rest is in not-split
        assert_eq!(files_not_to_split.len(), 4);

        // See layout of 2 set of files
        insta::assert_yaml_snapshot!(
            format_files_split("files to split:", &files_to_split.iter().map(|f| f.file.clone()).collect::<Vec<_>>(), "files not to split:", &files_not_to_split),
            @r###"
        ---
        - "files to split:"
        - "L0, all files 1b                                                                                                   "
        - "L0.1[450,620] 120s       |------------------------------------------L0.1------------------------------------------|"
        - "files not to split:"
        - "L0, all files 1b                                                                                                   "
        - "L0.2[650,750] 180s                                                    |------L0.2------|                           "
        - "L0.3[800,900] 300s                                                                               |------L0.3------|"
        - "L1, all files 1b                                                                                                   "
        - "L1.12[400,500] 60s       |-----L1.12------|                                                                        "
        - "L1.13[600,700] 60s                                           |-----L1.13------|                                    "
        "###
        );
    }
}
