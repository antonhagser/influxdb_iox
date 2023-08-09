//! layout tests for scenarios with large numbers of files
//!
//! See [crate::layout] module for detailed documentation

use data_types::CompactionLevel;
use iox_time::Time;
use std::time::Duration;

use crate::layouts::{layout_setup_builder, parquet_builder, run_layout_scenario, ONE_MB};

const MAX_DESIRED_FILE_SIZE: u64 = 100 * ONE_MB;

// This case simulates a backfill scenario with no existing data prior to the start of backfill.
//   - the customer starts backfilling yesterday's data, writing at random times spread across the day.
// The result:
//   - We get many L0s that each cover much of the day.
#[tokio::test]
async fn random_backfill_empty_partition() {
    test_helpers::maybe_start_logging();

    let setup = layout_setup_builder()
        .await
        .with_max_desired_file_size_bytes(MAX_DESIRED_FILE_SIZE)
        .with_max_num_files_per_plan(20)
        .build()
        .await;

    let num_tiny_l0_files = 50;
    let l0_size = MAX_DESIRED_FILE_SIZE / 10;

    // Assume the "day" is 1000 units of time, and spread the L0s across that
    for i in 0..num_tiny_l0_files {
        let i = i as i64;

        // Create a bit of variety in the start/stop times, but mostly they cover most of the day.
        let mut start_time = 50;
        let mut end_time = 950;
        match i % 4 {
            0 => {
                start_time += 26;
                end_time -= 18;
            }
            1 => {
                start_time -= 8;
                end_time += 36;
            }
            2 => {
                start_time += 123;
            }
            3 => {
                end_time -= 321;
            }
            _ => {}
        }

        setup
            .partition
            .create_parquet_file(
                parquet_builder()
                    .with_min_time(start_time)
                    .with_max_time(end_time)
                    .with_compaction_level(CompactionLevel::Initial)
                    .with_file_size_bytes(l0_size)
                    .with_max_l0_created_at(Time::from_timestamp_nanos(i + 1000)), // These files are created sequentially "today" with "yesterday's" data
            )
            .await;
    }

    // Add an extra file that doesn't overlap anything. Since this test case exercises high_l0_overlap_split, including an l0 file that overlaps nothing,
    // exercises a code path in high_l0_overlap_split where a file has no overlaps.
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(0)
                .with_max_time(1)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(l0_size)
                .with_max_l0_created_at(Time::from_timestamp_nanos(999)),
        )
        .await;

    insta::assert_yaml_snapshot!(
        run_layout_scenario(&setup).await,
        @r###"
    ---
    - "**** Input Files "
    - "L0, all files 10mb                                                                                                 "
    - "L0.1[76,932] 1us               |------------------------------------L0.1------------------------------------|      "
    - "L0.2[42,986] 1us            |----------------------------------------L0.2----------------------------------------| "
    - "L0.3[173,950] 1us                       |--------------------------------L0.3--------------------------------|     "
    - "L0.4[50,629] 1us             |-----------------------L0.4-----------------------|                                  "
    - "L0.5[76,932] 1us               |------------------------------------L0.5------------------------------------|      "
    - "L0.6[42,986] 1us            |----------------------------------------L0.6----------------------------------------| "
    - "L0.7[173,950] 1.01us                    |--------------------------------L0.7--------------------------------|     "
    - "L0.8[50,629] 1.01us          |-----------------------L0.8-----------------------|                                  "
    - "L0.9[76,932] 1.01us            |------------------------------------L0.9------------------------------------|      "
    - "L0.10[42,986] 1.01us        |---------------------------------------L0.10----------------------------------------| "
    - "L0.11[173,950] 1.01us                   |-------------------------------L0.11--------------------------------|     "
    - "L0.12[50,629] 1.01us         |----------------------L0.12-----------------------|                                  "
    - "L0.13[76,932] 1.01us           |-----------------------------------L0.13------------------------------------|      "
    - "L0.14[42,986] 1.01us        |---------------------------------------L0.14----------------------------------------| "
    - "L0.15[173,950] 1.01us                   |-------------------------------L0.15--------------------------------|     "
    - "L0.16[50,629] 1.01us         |----------------------L0.16-----------------------|                                  "
    - "L0.17[76,932] 1.02us           |-----------------------------------L0.17------------------------------------|      "
    - "L0.18[42,986] 1.02us        |---------------------------------------L0.18----------------------------------------| "
    - "L0.19[173,950] 1.02us                   |-------------------------------L0.19--------------------------------|     "
    - "L0.20[50,629] 1.02us         |----------------------L0.20-----------------------|                                  "
    - "L0.21[76,932] 1.02us           |-----------------------------------L0.21------------------------------------|      "
    - "L0.22[42,986] 1.02us        |---------------------------------------L0.22----------------------------------------| "
    - "L0.23[173,950] 1.02us                   |-------------------------------L0.23--------------------------------|     "
    - "L0.24[50,629] 1.02us         |----------------------L0.24-----------------------|                                  "
    - "L0.25[76,932] 1.02us           |-----------------------------------L0.25------------------------------------|      "
    - "L0.26[42,986] 1.02us        |---------------------------------------L0.26----------------------------------------| "
    - "L0.27[173,950] 1.03us                   |-------------------------------L0.27--------------------------------|     "
    - "L0.28[50,629] 1.03us         |----------------------L0.28-----------------------|                                  "
    - "L0.29[76,932] 1.03us           |-----------------------------------L0.29------------------------------------|      "
    - "L0.30[42,986] 1.03us        |---------------------------------------L0.30----------------------------------------| "
    - "L0.31[173,950] 1.03us                   |-------------------------------L0.31--------------------------------|     "
    - "L0.32[50,629] 1.03us         |----------------------L0.32-----------------------|                                  "
    - "L0.33[76,932] 1.03us           |-----------------------------------L0.33------------------------------------|      "
    - "L0.34[42,986] 1.03us        |---------------------------------------L0.34----------------------------------------| "
    - "L0.35[173,950] 1.03us                   |-------------------------------L0.35--------------------------------|     "
    - "L0.36[50,629] 1.03us         |----------------------L0.36-----------------------|                                  "
    - "L0.37[76,932] 1.04us           |-----------------------------------L0.37------------------------------------|      "
    - "L0.38[42,986] 1.04us        |---------------------------------------L0.38----------------------------------------| "
    - "L0.39[173,950] 1.04us                   |-------------------------------L0.39--------------------------------|     "
    - "L0.40[50,629] 1.04us         |----------------------L0.40-----------------------|                                  "
    - "L0.41[76,932] 1.04us           |-----------------------------------L0.41------------------------------------|      "
    - "L0.42[42,986] 1.04us        |---------------------------------------L0.42----------------------------------------| "
    - "L0.43[173,950] 1.04us                   |-------------------------------L0.43--------------------------------|     "
    - "L0.44[50,629] 1.04us         |----------------------L0.44-----------------------|                                  "
    - "L0.45[76,932] 1.04us           |-----------------------------------L0.45------------------------------------|      "
    - "L0.46[42,986] 1.05us        |---------------------------------------L0.46----------------------------------------| "
    - "L0.47[173,950] 1.05us                   |-------------------------------L0.47--------------------------------|     "
    - "L0.48[50,629] 1.05us         |----------------------L0.48-----------------------|                                  "
    - "L0.49[76,932] 1.05us           |-----------------------------------L0.49------------------------------------|      "
    - "L0.50[42,986] 1.05us        |---------------------------------------L0.50----------------------------------------| "
    - "L0.51[0,1] 999ns         |L0.51|                                                                                   "
    - "**** Simulation run 0, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.1[76,932] 1us         |------------------------------------------L0.1------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 4mb                                 |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1us 3mb                                                                  |----------L0.?-----------| "
    - "**** Simulation run 1, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.2[42,986] 1us         |------------------------------------------L0.2------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 3mb                                  |-----------L0.?------------|                               "
    - "L0.?[671,986] 1us 3mb                                                               |------------L0.?------------| "
    - "**** Simulation run 2, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.3[173,950] 1us        |------------------------------------------L0.3------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1us 2mb    |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1us 4mb                         |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1us 4mb                                                             |-------------L0.?-------------| "
    - "**** Simulation run 3, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.4[50,629] 1us         |------------------------------------------L0.4------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1us 5mb     |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1us 5mb                                                   |------------------L0.?------------------| "
    - "**** Simulation run 4, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.5[76,932] 1us         |------------------------------------------L0.5------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 4mb                                 |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1us 3mb                                                                  |----------L0.?-----------| "
    - "**** Simulation run 5, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.6[42,986] 1us         |------------------------------------------L0.6------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 3mb                                  |-----------L0.?------------|                               "
    - "L0.?[671,986] 1us 3mb                                                               |------------L0.?------------| "
    - "**** Simulation run 6, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.7[173,950] 1.01us     |------------------------------------------L0.7------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 7, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.8[50,629] 1.01us      |------------------------------------------L0.8------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 8, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.9[76,932] 1.01us      |------------------------------------------L0.9------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.01us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 9, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.10[42,986] 1.01us     |-----------------------------------------L0.10------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.01us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 10, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.11[173,950] 1.01us    |-----------------------------------------L0.11------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 11, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.12[50,629] 1.01us     |-----------------------------------------L0.12------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 12, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.13[76,932] 1.01us     |-----------------------------------------L0.13------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.01us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 13, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.14[42,986] 1.01us     |-----------------------------------------L0.14------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.01us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 14, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.15[173,950] 1.01us    |-----------------------------------------L0.15------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 15, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.16[50,629] 1.01us     |-----------------------------------------L0.16------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 16, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.17[76,932] 1.02us     |-----------------------------------------L0.17------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 17, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.18[42,986] 1.02us     |-----------------------------------------L0.18------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 18, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.19[173,950] 1.02us    |-----------------------------------------L0.19------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.02us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.02us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.02us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 19, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.20[50,629] 1.02us     |-----------------------------------------L0.20------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.02us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.02us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 20, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.21[76,932] 1.02us     |-----------------------------------------L0.21------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 21, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.22[42,986] 1.02us     |-----------------------------------------L0.22------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 22, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.23[173,950] 1.02us    |-----------------------------------------L0.23------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.02us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.02us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.02us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 23, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.24[50,629] 1.02us     |-----------------------------------------L0.24------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.02us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.02us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 24, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.25[76,932] 1.02us     |-----------------------------------------L0.25------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 25, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.26[42,986] 1.02us     |-----------------------------------------L0.26------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 26, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.27[173,950] 1.03us    |-----------------------------------------L0.27------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 27, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.28[50,629] 1.03us     |-----------------------------------------L0.28------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 28, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.29[76,932] 1.03us     |-----------------------------------------L0.29------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.03us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 29, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.30[42,986] 1.03us     |-----------------------------------------L0.30------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.03us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 30, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.31[173,950] 1.03us    |-----------------------------------------L0.31------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 31, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.32[50,629] 1.03us     |-----------------------------------------L0.32------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 32, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.33[76,932] 1.03us     |-----------------------------------------L0.33------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.03us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 33, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.34[42,986] 1.03us     |-----------------------------------------L0.34------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.03us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 34, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.35[173,950] 1.03us    |-----------------------------------------L0.35------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 35, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.36[50,629] 1.03us     |-----------------------------------------L0.36------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 36, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.37[76,932] 1.04us     |-----------------------------------------L0.37------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 37, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.38[42,986] 1.04us     |-----------------------------------------L0.38------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.04us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 38, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.39[173,950] 1.04us    |-----------------------------------------L0.39------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.04us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.04us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.04us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 39, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.40[50,629] 1.04us     |-----------------------------------------L0.40------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.04us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.04us 5mb                                                |------------------L0.?------------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 40 files: L0.1, L0.2, L0.3, L0.4, L0.5, L0.6, L0.7, L0.8, L0.9, L0.10, L0.11, L0.12, L0.13, L0.14, L0.15, L0.16, L0.17, L0.18, L0.19, L0.20, L0.21, L0.22, L0.23, L0.24, L0.25, L0.26, L0.27, L0.28, L0.29, L0.30, L0.31, L0.32, L0.33, L0.34, L0.35, L0.36, L0.37, L0.38, L0.39, L0.40"
    - "  Creating 110 files"
    - "**** Simulation run 40, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.41[76,932] 1.04us     |-----------------------------------------L0.41------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 41, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.42[42,986] 1.04us     |-----------------------------------------L0.42------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.04us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 42, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.43[173,950] 1.04us    |-----------------------------------------L0.43------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.04us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.04us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.04us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 43, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.44[50,629] 1.04us     |-----------------------------------------L0.44------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.04us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.04us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 44, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.45[76,932] 1.04us     |-----------------------------------------L0.45------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 45, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.46[42,986] 1.05us     |-----------------------------------------L0.46------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.05us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 46, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.47[173,950] 1.05us    |-----------------------------------------L0.47------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.05us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.05us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.05us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 47, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.48[50,629] 1.05us     |-----------------------------------------L0.48------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.05us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.05us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 48, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.49[76,932] 1.05us     |-----------------------------------------L0.49------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.05us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 49, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.50[42,986] 1.05us     |-----------------------------------------L0.50------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.05us 3mb                                                            |------------L0.?------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.41, L0.42, L0.43, L0.44, L0.45, L0.46, L0.47, L0.48, L0.49, L0.50"
    - "  Creating 28 files"
    - "**** Simulation run 50, type=compact(ManySmallFiles). 20 Input Files, 76mb total:"
    - "L0                                                                                                                 "
    - "L0.51[0,1] 999ns 10mb    |L0.51|                                                                                   "
    - "L0.52[76,356] 1us 3mb                       |-------------------------------L0.52--------------------------------| "
    - "L0.55[42,356] 1us 3mb              |------------------------------------L0.55------------------------------------| "
    - "L0.58[173,356] 1us 2mb                                              |-------------------L0.58--------------------| "
    - "L0.61[50,356] 1us 5mb                |-----------------------------------L0.61-----------------------------------| "
    - "L0.63[76,356] 1us 3mb                       |-------------------------------L0.63--------------------------------| "
    - "L0.66[42,356] 1us 3mb              |------------------------------------L0.66------------------------------------| "
    - "L0.69[173,356] 1.01us 2mb                                           |-------------------L0.69--------------------| "
    - "L0.72[50,356] 1.01us 5mb             |-----------------------------------L0.72-----------------------------------| "
    - "L0.74[76,356] 1.01us 3mb                    |-------------------------------L0.74--------------------------------| "
    - "L0.77[42,356] 1.01us 3mb           |------------------------------------L0.77------------------------------------| "
    - "L0.80[173,356] 1.01us 2mb                                           |-------------------L0.80--------------------| "
    - "L0.83[50,356] 1.01us 5mb             |-----------------------------------L0.83-----------------------------------| "
    - "L0.85[76,356] 1.01us 3mb                    |-------------------------------L0.85--------------------------------| "
    - "L0.88[42,356] 1.01us 3mb           |------------------------------------L0.88------------------------------------| "
    - "L0.91[173,356] 1.01us 2mb                                           |-------------------L0.91--------------------| "
    - "L0.94[50,356] 1.01us 5mb             |-----------------------------------L0.94-----------------------------------| "
    - "L0.96[76,356] 1.02us 3mb                    |-------------------------------L0.96--------------------------------| "
    - "L0.99[42,356] 1.02us 3mb           |------------------------------------L0.99------------------------------------| "
    - "L0.102[173,356] 1.02us 2mb                                           |-------------------L0.102-------------------| "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 76mb total:"
    - "L0, all files 76mb                                                                                                 "
    - "L0.?[0,356] 1.02us       |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 20 files: L0.51, L0.52, L0.55, L0.58, L0.61, L0.63, L0.66, L0.69, L0.72, L0.74, L0.77, L0.80, L0.83, L0.85, L0.88, L0.91, L0.94, L0.96, L0.99, L0.102"
    - "  Creating 1 files"
    - "**** Simulation run 51, type=compact(ManySmallFiles). 20 Input Files, 71mb total:"
    - "L0                                                                                                                 "
    - "L0.105[50,356] 1.02us 5mb  |---------------------------------------L0.105----------------------------------------| "
    - "L0.107[76,356] 1.02us 3mb         |------------------------------------L0.107------------------------------------| "
    - "L0.110[42,356] 1.02us 3mb|-----------------------------------------L0.110-----------------------------------------|"
    - "L0.113[173,356] 1.02us 2mb                                     |----------------------L0.113----------------------| "
    - "L0.116[50,356] 1.02us 5mb  |---------------------------------------L0.116----------------------------------------| "
    - "L0.118[76,356] 1.02us 3mb         |------------------------------------L0.118------------------------------------| "
    - "L0.121[42,356] 1.02us 3mb|-----------------------------------------L0.121-----------------------------------------|"
    - "L0.124[173,356] 1.03us 2mb                                     |----------------------L0.124----------------------| "
    - "L0.127[50,356] 1.03us 5mb  |---------------------------------------L0.127----------------------------------------| "
    - "L0.129[76,356] 1.03us 3mb         |------------------------------------L0.129------------------------------------| "
    - "L0.132[42,356] 1.03us 3mb|-----------------------------------------L0.132-----------------------------------------|"
    - "L0.135[173,356] 1.03us 2mb                                     |----------------------L0.135----------------------| "
    - "L0.138[50,356] 1.03us 5mb  |---------------------------------------L0.138----------------------------------------| "
    - "L0.140[76,356] 1.03us 3mb         |------------------------------------L0.140------------------------------------| "
    - "L0.143[42,356] 1.03us 3mb|-----------------------------------------L0.143-----------------------------------------|"
    - "L0.146[173,356] 1.03us 2mb                                     |----------------------L0.146----------------------| "
    - "L0.149[50,356] 1.03us 5mb  |---------------------------------------L0.149----------------------------------------| "
    - "L0.151[76,356] 1.04us 3mb         |------------------------------------L0.151------------------------------------| "
    - "L0.154[42,356] 1.04us 3mb|-----------------------------------------L0.154-----------------------------------------|"
    - "L0.157[173,356] 1.04us 2mb                                     |----------------------L0.157----------------------| "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 71mb total:"
    - "L0, all files 71mb                                                                                                 "
    - "L0.?[42,356] 1.04us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 20 files: L0.105, L0.107, L0.110, L0.113, L0.116, L0.118, L0.121, L0.124, L0.127, L0.129, L0.132, L0.135, L0.138, L0.140, L0.143, L0.146, L0.149, L0.151, L0.154, L0.157"
    - "  Creating 1 files"
    - "**** Simulation run 52, type=compact(ManySmallFiles). 11 Input Files, 40mb total:"
    - "L0                                                                                                                 "
    - "L0.160[50,356] 1.04us 5mb  |---------------------------------------L0.160----------------------------------------| "
    - "L0.162[76,356] 1.04us 3mb         |------------------------------------L0.162------------------------------------| "
    - "L0.165[42,356] 1.04us 3mb|-----------------------------------------L0.165-----------------------------------------|"
    - "L0.168[173,356] 1.04us 2mb                                     |----------------------L0.168----------------------| "
    - "L0.171[50,356] 1.04us 5mb  |---------------------------------------L0.171----------------------------------------| "
    - "L0.173[76,356] 1.04us 3mb         |------------------------------------L0.173------------------------------------| "
    - "L0.176[42,356] 1.05us 3mb|-----------------------------------------L0.176-----------------------------------------|"
    - "L0.179[173,356] 1.05us 2mb                                     |----------------------L0.179----------------------| "
    - "L0.182[50,356] 1.05us 5mb  |---------------------------------------L0.182----------------------------------------| "
    - "L0.184[76,356] 1.05us 3mb         |------------------------------------L0.184------------------------------------| "
    - "L0.187[42,356] 1.05us 3mb|-----------------------------------------L0.187-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 40mb total:"
    - "L0, all files 40mb                                                                                                 "
    - "L0.?[42,356] 1.05us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 11 files: L0.160, L0.162, L0.165, L0.168, L0.171, L0.173, L0.176, L0.179, L0.182, L0.184, L0.187"
    - "  Creating 1 files"
    - "**** Simulation run 53, type=compact(ManySmallFiles). 20 Input Files, 79mb total:"
    - "L0                                                                                                                 "
    - "L0.53[357,670] 1us 4mb   |-----------------------------------------L0.53------------------------------------------|"
    - "L0.56[357,670] 1us 3mb   |-----------------------------------------L0.56------------------------------------------|"
    - "L0.59[357,670] 1us 4mb   |-----------------------------------------L0.59------------------------------------------|"
    - "L0.62[357,629] 1us 5mb   |-----------------------------------L0.62------------------------------------|            "
    - "L0.64[357,670] 1us 4mb   |-----------------------------------------L0.64------------------------------------------|"
    - "L0.67[357,670] 1us 3mb   |-----------------------------------------L0.67------------------------------------------|"
    - "L0.70[357,670] 1.01us 4mb|-----------------------------------------L0.70------------------------------------------|"
    - "L0.73[357,629] 1.01us 5mb|-----------------------------------L0.73------------------------------------|            "
    - "L0.75[357,670] 1.01us 4mb|-----------------------------------------L0.75------------------------------------------|"
    - "L0.78[357,670] 1.01us 3mb|-----------------------------------------L0.78------------------------------------------|"
    - "L0.81[357,670] 1.01us 4mb|-----------------------------------------L0.81------------------------------------------|"
    - "L0.84[357,629] 1.01us 5mb|-----------------------------------L0.84------------------------------------|            "
    - "L0.86[357,670] 1.01us 4mb|-----------------------------------------L0.86------------------------------------------|"
    - "L0.89[357,670] 1.01us 3mb|-----------------------------------------L0.89------------------------------------------|"
    - "L0.92[357,670] 1.01us 4mb|-----------------------------------------L0.92------------------------------------------|"
    - "L0.95[357,629] 1.01us 5mb|-----------------------------------L0.95------------------------------------|            "
    - "L0.97[357,670] 1.02us 4mb|-----------------------------------------L0.97------------------------------------------|"
    - "L0.100[357,670] 1.02us 3mb|-----------------------------------------L0.100-----------------------------------------|"
    - "L0.103[357,670] 1.02us 4mb|-----------------------------------------L0.103-----------------------------------------|"
    - "L0.106[357,629] 1.02us 5mb|-----------------------------------L0.106-----------------------------------|            "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 79mb total:"
    - "L0, all files 79mb                                                                                                 "
    - "L0.?[357,670] 1.02us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 20 files: L0.53, L0.56, L0.59, L0.62, L0.64, L0.67, L0.70, L0.73, L0.75, L0.78, L0.81, L0.84, L0.86, L0.89, L0.92, L0.95, L0.97, L0.100, L0.103, L0.106"
    - "  Creating 1 files"
    - "**** Simulation run 54, type=compact(ManySmallFiles). 20 Input Files, 79mb total:"
    - "L0                                                                                                                 "
    - "L0.108[357,670] 1.02us 4mb|-----------------------------------------L0.108-----------------------------------------|"
    - "L0.111[357,670] 1.02us 3mb|-----------------------------------------L0.111-----------------------------------------|"
    - "L0.114[357,670] 1.02us 4mb|-----------------------------------------L0.114-----------------------------------------|"
    - "L0.117[357,629] 1.02us 5mb|-----------------------------------L0.117-----------------------------------|            "
    - "L0.119[357,670] 1.02us 4mb|-----------------------------------------L0.119-----------------------------------------|"
    - "L0.122[357,670] 1.02us 3mb|-----------------------------------------L0.122-----------------------------------------|"
    - "L0.125[357,670] 1.03us 4mb|-----------------------------------------L0.125-----------------------------------------|"
    - "L0.128[357,629] 1.03us 5mb|-----------------------------------L0.128-----------------------------------|            "
    - "L0.130[357,670] 1.03us 4mb|-----------------------------------------L0.130-----------------------------------------|"
    - "L0.133[357,670] 1.03us 3mb|-----------------------------------------L0.133-----------------------------------------|"
    - "L0.136[357,670] 1.03us 4mb|-----------------------------------------L0.136-----------------------------------------|"
    - "L0.139[357,629] 1.03us 5mb|-----------------------------------L0.139-----------------------------------|            "
    - "L0.141[357,670] 1.03us 4mb|-----------------------------------------L0.141-----------------------------------------|"
    - "L0.144[357,670] 1.03us 3mb|-----------------------------------------L0.144-----------------------------------------|"
    - "L0.147[357,670] 1.03us 4mb|-----------------------------------------L0.147-----------------------------------------|"
    - "L0.150[357,629] 1.03us 5mb|-----------------------------------L0.150-----------------------------------|            "
    - "L0.152[357,670] 1.04us 4mb|-----------------------------------------L0.152-----------------------------------------|"
    - "L0.155[357,670] 1.04us 3mb|-----------------------------------------L0.155-----------------------------------------|"
    - "L0.158[357,670] 1.04us 4mb|-----------------------------------------L0.158-----------------------------------------|"
    - "L0.161[357,629] 1.04us 5mb|-----------------------------------L0.161-----------------------------------|            "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 79mb total:"
    - "L0, all files 79mb                                                                                                 "
    - "L0.?[357,670] 1.04us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 20 files: L0.108, L0.111, L0.114, L0.117, L0.119, L0.122, L0.125, L0.128, L0.130, L0.133, L0.136, L0.139, L0.141, L0.144, L0.147, L0.150, L0.152, L0.155, L0.158, L0.161"
    - "  Creating 1 files"
    - "**** Simulation run 55, type=compact(ManySmallFiles). 20 Input Files, 67mb total:"
    - "L0                                                                                                                 "
    - "L0.54[671,932] 1us 3mb   |---------------------------------L0.54----------------------------------|                "
    - "L0.57[671,986] 1us 3mb   |-----------------------------------------L0.57------------------------------------------|"
    - "L0.60[671,950] 1us 4mb   |------------------------------------L0.60------------------------------------|           "
    - "L0.65[671,932] 1us 3mb   |---------------------------------L0.65----------------------------------|                "
    - "L0.68[671,986] 1us 3mb   |-----------------------------------------L0.68------------------------------------------|"
    - "L0.71[671,950] 1.01us 4mb|------------------------------------L0.71------------------------------------|           "
    - "L0.76[671,932] 1.01us 3mb|---------------------------------L0.76----------------------------------|                "
    - "L0.79[671,986] 1.01us 3mb|-----------------------------------------L0.79------------------------------------------|"
    - "L0.82[671,950] 1.01us 4mb|------------------------------------L0.82------------------------------------|           "
    - "L0.87[671,932] 1.01us 3mb|---------------------------------L0.87----------------------------------|                "
    - "L0.90[671,986] 1.01us 3mb|-----------------------------------------L0.90------------------------------------------|"
    - "L0.93[671,950] 1.01us 4mb|------------------------------------L0.93------------------------------------|           "
    - "L0.98[671,932] 1.02us 3mb|---------------------------------L0.98----------------------------------|                "
    - "L0.101[671,986] 1.02us 3mb|-----------------------------------------L0.101-----------------------------------------|"
    - "L0.104[671,950] 1.02us 4mb|-----------------------------------L0.104------------------------------------|           "
    - "L0.109[671,932] 1.02us 3mb|---------------------------------L0.109---------------------------------|                "
    - "L0.112[671,986] 1.02us 3mb|-----------------------------------------L0.112-----------------------------------------|"
    - "L0.115[671,950] 1.02us 4mb|-----------------------------------L0.115------------------------------------|           "
    - "L0.120[671,932] 1.02us 3mb|---------------------------------L0.120---------------------------------|                "
    - "L0.123[671,986] 1.02us 3mb|-----------------------------------------L0.123-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 67mb total:"
    - "L0, all files 67mb                                                                                                 "
    - "L0.?[671,986] 1.02us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 20 files: L0.54, L0.57, L0.60, L0.65, L0.68, L0.71, L0.76, L0.79, L0.82, L0.87, L0.90, L0.93, L0.98, L0.101, L0.104, L0.109, L0.112, L0.115, L0.120, L0.123"
    - "  Creating 1 files"
    - "**** Simulation run 56, type=compact(ManySmallFiles). 18 Input Files, 60mb total:"
    - "L0                                                                                                                 "
    - "L0.126[671,950] 1.03us 4mb|-----------------------------------L0.126------------------------------------|           "
    - "L0.131[671,932] 1.03us 3mb|---------------------------------L0.131---------------------------------|                "
    - "L0.134[671,986] 1.03us 3mb|-----------------------------------------L0.134-----------------------------------------|"
    - "L0.137[671,950] 1.03us 4mb|-----------------------------------L0.137------------------------------------|           "
    - "L0.142[671,932] 1.03us 3mb|---------------------------------L0.142---------------------------------|                "
    - "L0.145[671,986] 1.03us 3mb|-----------------------------------------L0.145-----------------------------------------|"
    - "L0.148[671,950] 1.03us 4mb|-----------------------------------L0.148------------------------------------|           "
    - "L0.153[671,932] 1.04us 3mb|---------------------------------L0.153---------------------------------|                "
    - "L0.156[671,986] 1.04us 3mb|-----------------------------------------L0.156-----------------------------------------|"
    - "L0.159[671,950] 1.04us 4mb|-----------------------------------L0.159------------------------------------|           "
    - "L0.164[671,932] 1.04us 3mb|---------------------------------L0.164---------------------------------|                "
    - "L0.167[671,986] 1.04us 3mb|-----------------------------------------L0.167-----------------------------------------|"
    - "L0.170[671,950] 1.04us 4mb|-----------------------------------L0.170------------------------------------|           "
    - "L0.175[671,932] 1.04us 3mb|---------------------------------L0.175---------------------------------|                "
    - "L0.178[671,986] 1.05us 3mb|-----------------------------------------L0.178-----------------------------------------|"
    - "L0.181[671,950] 1.05us 4mb|-----------------------------------L0.181------------------------------------|           "
    - "L0.186[671,932] 1.05us 3mb|---------------------------------L0.186---------------------------------|                "
    - "L0.189[671,986] 1.05us 3mb|-----------------------------------------L0.189-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 60mb total:"
    - "L0, all files 60mb                                                                                                 "
    - "L0.?[671,986] 1.05us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 18 files: L0.126, L0.131, L0.134, L0.137, L0.142, L0.145, L0.148, L0.153, L0.156, L0.159, L0.164, L0.167, L0.170, L0.175, L0.178, L0.181, L0.186, L0.189"
    - "  Creating 1 files"
    - "**** Simulation run 57, type=compact(ManySmallFiles). 10 Input Files, 38mb total:"
    - "L0                                                                                                                 "
    - "L0.163[357,670] 1.04us 4mb|-----------------------------------------L0.163-----------------------------------------|"
    - "L0.166[357,670] 1.04us 3mb|-----------------------------------------L0.166-----------------------------------------|"
    - "L0.169[357,670] 1.04us 4mb|-----------------------------------------L0.169-----------------------------------------|"
    - "L0.172[357,629] 1.04us 5mb|-----------------------------------L0.172-----------------------------------|            "
    - "L0.174[357,670] 1.04us 4mb|-----------------------------------------L0.174-----------------------------------------|"
    - "L0.177[357,670] 1.05us 3mb|-----------------------------------------L0.177-----------------------------------------|"
    - "L0.180[357,670] 1.05us 4mb|-----------------------------------------L0.180-----------------------------------------|"
    - "L0.183[357,629] 1.05us 5mb|-----------------------------------L0.183-----------------------------------|            "
    - "L0.185[357,670] 1.05us 4mb|-----------------------------------------L0.185-----------------------------------------|"
    - "L0.188[357,670] 1.05us 3mb|-----------------------------------------L0.188-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.?[357,670] 1.05us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.163, L0.166, L0.169, L0.172, L0.174, L0.177, L0.180, L0.183, L0.185, L0.188"
    - "  Creating 1 files"
    - "**** Simulation run 58, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[338, 676]). 4 Input Files, 292mb total:"
    - "L0                                                                                                                 "
    - "L0.190[0,356] 1.02us 76mb|------------L0.190------------|                                                          "
    - "L0.193[357,670] 1.02us 79mb                                |----------L0.193----------|                              "
    - "L0.195[671,986] 1.02us 67mb                                                             |----------L0.195----------| "
    - "L0.191[42,356] 1.04us 71mb   |----------L0.191----------|                                                           "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 292mb total:"
    - "L1                                                                                                                 "
    - "L1.?[0,338] 1.04us 100mb |------------L1.?------------|                                                            "
    - "L1.?[339,676] 1.04us 100mb                              |------------L1.?------------|                              "
    - "L1.?[677,986] 1.04us 92mb                                                             |-----------L1.?-----------| "
    - "Committing partition 1:"
    - "  Soft Deleting 4 files: L0.190, L0.191, L0.193, L0.195"
    - "  Creating 3 files"
    - "**** Simulation run 59, type=split(ReduceOverlap)(split_times=[676]). 1 Input Files, 60mb total:"
    - "L0, all files 60mb                                                                                                 "
    - "L0.196[671,986] 1.05us   |-----------------------------------------L0.196-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 60mb total:"
    - "L0                                                                                                                 "
    - "L0.?[671,676] 1.05us 980kb|L0.?|                                                                                    "
    - "L0.?[677,986] 1.05us 59mb |-----------------------------------------L0.?-----------------------------------------| "
    - "**** Simulation run 60, type=split(ReduceOverlap)(split_times=[338]). 1 Input Files, 40mb total:"
    - "L0, all files 40mb                                                                                                 "
    - "L0.192[42,356] 1.05us    |-----------------------------------------L0.192-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 40mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,338] 1.05us 38mb |---------------------------------------L0.?---------------------------------------|      "
    - "L0.?[339,356] 1.05us 2mb                                                                                      |L0.?|"
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L0.192, L0.196"
    - "  Creating 4 files"
    - "**** Simulation run 61, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[528]). 2 Input Files, 179mb total:"
    - "L0                                                                                                                 "
    - "L0.194[357,670] 1.04us 79mb    |-------------------------------------L0.194--------------------------------------|   "
    - "L1                                                                                                                 "
    - "L1.199[339,676] 1.04us 100mb|-----------------------------------------L1.199-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 179mb total:"
    - "L1                                                                                                                 "
    - "L1.?[339,528] 1.04us 100mb|----------------------L1.?----------------------|                                        "
    - "L1.?[529,676] 1.04us 78mb                                                  |----------------L1.?-----------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L0.194, L1.199"
    - "  Creating 2 files"
    - "**** Simulation run 62, type=split(ReduceOverlap)(split_times=[528]). 1 Input Files, 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.197[357,670] 1.05us   |-----------------------------------------L0.197-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0                                                                                                                 "
    - "L0.?[357,528] 1.05us 21mb|---------------------L0.?----------------------|                                         "
    - "L0.?[529,670] 1.05us 17mb                                                 |-----------------L0.?-----------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 1 files: L0.197"
    - "  Creating 2 files"
    - "**** Simulation run 63, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[202, 404]). 5 Input Files, 262mb total:"
    - "L0                                                                                                                 "
    - "L0.203[42,338] 1.05us 38mb       |---------------------L0.203---------------------|                                 "
    - "L0.204[339,356] 1.05us 2mb                                                         |L0.204|                         "
    - "L0.207[357,528] 1.05us 21mb                                                            |----------L0.207-----------| "
    - "L1                                                                                                                 "
    - "L1.198[0,338] 1.04us 100mb|------------------------L1.198-------------------------|                                 "
    - "L1.205[339,528] 1.04us 100mb                                                         |------------L1.205------------| "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 262mb total:"
    - "L1                                                                                                                 "
    - "L1.?[0,202] 1.05us 100mb |--------------L1.?--------------|                                                        "
    - "L1.?[203,404] 1.05us 100mb                                  |--------------L1.?--------------|                      "
    - "L1.?[405,528] 1.05us 62mb                                                                     |-------L1.?-------| "
    - "Committing partition 1:"
    - "  Soft Deleting 5 files: L1.198, L0.203, L0.204, L1.205, L0.207"
    - "  Creating 3 files"
    - "**** Simulation run 64, type=split(CompactAndSplitOutput(TotalSizeLessThanMaxCompactSize))(split_times=[714, 899]). 5 Input Files, 248mb total:"
    - "L0                                                                                                                 "
    - "L0.202[677,986] 1.05us 59mb                             |--------------------------L0.202--------------------------| "
    - "L0.201[671,676] 1.05us 980kb                           |L0.201|                                                       "
    - "L0.208[529,670] 1.05us 17mb|---------L0.208----------|                                                               "
    - "L1                                                                                                                 "
    - "L1.200[677,986] 1.04us 92mb                             |--------------------------L1.200--------------------------| "
    - "L1.206[529,676] 1.04us 78mb|----------L1.206----------|                                                              "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 248mb total:"
    - "L1                                                                                                                 "
    - "L1.?[529,714] 1.05us 101mb|---------------L1.?---------------|                                                      "
    - "L1.?[715,899] 1.05us 100mb                                    |---------------L1.?---------------|                  "
    - "L1.?[900,986] 1.05us 48mb                                                                         |-----L1.?-----| "
    - "Committing partition 1:"
    - "  Soft Deleting 5 files: L1.200, L0.201, L0.202, L1.206, L0.208"
    - "  Creating 3 files"
    - "**** Simulation run 65, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[398, 593]). 3 Input Files, 262mb total:"
    - "L1                                                                                                                 "
    - "L1.210[203,404] 1.05us 100mb|-------------L1.210--------------|                                                       "
    - "L1.211[405,528] 1.05us 62mb                                   |------L1.211-------|                                  "
    - "L1.212[529,714] 1.05us 101mb                                                         |------------L1.212------------| "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 262mb total:"
    - "L2                                                                                                                 "
    - "L2.?[203,398] 1.05us 100mb|--------------L2.?--------------|                                                        "
    - "L2.?[399,593] 1.05us 100mb                                  |--------------L2.?--------------|                      "
    - "L2.?[594,714] 1.05us 63mb                                                                    |-------L2.?--------| "
    - "Committing partition 1:"
    - "  Soft Deleting 3 files: L1.210, L1.211, L1.212"
    - "  Upgrading 1 files level to CompactionLevel::L2: L1.209"
    - "  Creating 3 files"
    - "**** Simulation run 66, type=split(CompactAndSplitOutput(TotalSizeLessThanMaxCompactSize))(split_times=[899]). 2 Input Files, 148mb total:"
    - "L1                                                                                                                 "
    - "L1.214[900,986] 1.05us 48mb                                                             |----------L1.214----------| "
    - "L1.213[715,899] 1.05us 100mb|--------------------------L1.213---------------------------|                             "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 148mb total:"
    - "L2                                                                                                                 "
    - "L2.?[715,899] 1.05us 100mb|---------------------------L2.?----------------------------|                             "
    - "L2.?[900,986] 1.05us 47mb                                                             |-----------L2.?-----------| "
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L1.213, L1.214"
    - "  Creating 2 files"
    - "**** Final Output Files (2.48gb written)"
    - "L2                                                                                                                 "
    - "L2.209[0,202] 1.05us 100mb|-----L2.209-----|                                                                        "
    - "L2.215[203,398] 1.05us 100mb                  |----L2.215-----|                                                       "
    - "L2.216[399,593] 1.05us 100mb                                    |----L2.216-----|                                     "
    - "L2.217[594,714] 1.05us 63mb                                                      |-L2.217-|                          "
    - "L2.218[715,899] 1.05us 100mb                                                                 |----L2.218----|         "
    - "L2.219[900,986] 1.05us 47mb                                                                                  |L2.219|"
    "###
    );
}

// This case simulates a backfill scenario with existing data prior to the start of backfill.
//   - we have L2s covering the whole time range of yesterday
//   - the customer starts backfilling more of yesterday's data, writing at random times spread across the day.
// The result:
//   - We start with compacted L2s covering the day, then get many L0s that each cover much of the day.
#[tokio::test]
async fn random_backfill_over_l2s() {
    test_helpers::maybe_start_logging();

    let setup = layout_setup_builder()
        .await
        // compact at most 10 L0 files per plan
        .with_max_num_files_per_plan(10)
        .with_max_desired_file_size_bytes(MAX_DESIRED_FILE_SIZE)
        .with_partition_timeout(Duration::from_secs(10))
        .build()
        .await;

    let day = 1000;
    let num_l2_files = 10;
    let l2_time = day / num_l2_files;
    let num_tiny_l0_files = 50;
    let l2_size = MAX_DESIRED_FILE_SIZE;
    let l0_size = MAX_DESIRED_FILE_SIZE / 10;

    for i in 0..num_l2_files {
        setup
            .partition
            .create_parquet_file(
                parquet_builder()
                    .with_min_time(i * l2_time)
                    .with_max_time((i + 1) * l2_time - 1)
                    .with_compaction_level(CompactionLevel::Final)
                    .with_file_size_bytes(l2_size)
                    .with_max_l0_created_at(Time::from_timestamp_nanos((i + 1) * l2_time - 1)), // These files are created sequentially "yesterday" with "yesterday's" data
            )
            .await;
    }

    // Assume the "day" is 1000 units of time, make the L0s span most of the day, with a little variability.
    for i in 0..num_tiny_l0_files {
        let i = i as i64;

        let mut start_time = 50;
        let mut end_time = 950;
        match i % 4 {
            0 => {
                start_time += 26;
                end_time -= 18;
            }
            1 => {
                start_time -= 8;
                end_time += 36;
            }
            2 => {
                start_time += 123;
            }
            3 => {
                end_time -= 321;
            }
            _ => {}
        }

        setup
            .partition
            .create_parquet_file(
                parquet_builder()
                    .with_min_time(start_time)
                    .with_max_time(end_time)
                    .with_compaction_level(CompactionLevel::Initial)
                    .with_file_size_bytes(l0_size)
                    .with_max_l0_created_at(Time::from_timestamp_nanos(i + 1000)), // These files are created sequentially "today" with "yesterday's" data
            )
            .await;
    }

    insta::assert_yaml_snapshot!(
        run_layout_scenario(&setup).await,
        @r###"
    ---
    - "**** Input Files "
    - "L0                                                                                                                 "
    - "L0.11[76,932] 1us 10mb         |-----------------------------------L0.11-----------------------------------|       "
    - "L0.12[42,986] 1us 10mb      |---------------------------------------L0.12---------------------------------------|  "
    - "L0.13[173,950] 1us 10mb                 |-------------------------------L0.13--------------------------------|     "
    - "L0.14[50,629] 1us 10mb       |----------------------L0.14-----------------------|                                  "
    - "L0.15[76,932] 1us 10mb         |-----------------------------------L0.15-----------------------------------|       "
    - "L0.16[42,986] 1us 10mb      |---------------------------------------L0.16---------------------------------------|  "
    - "L0.17[173,950] 1.01us 10mb               |-------------------------------L0.17--------------------------------|     "
    - "L0.18[50,629] 1.01us 10mb    |----------------------L0.18-----------------------|                                  "
    - "L0.19[76,932] 1.01us 10mb      |-----------------------------------L0.19-----------------------------------|       "
    - "L0.20[42,986] 1.01us 10mb   |---------------------------------------L0.20---------------------------------------|  "
    - "L0.21[173,950] 1.01us 10mb               |-------------------------------L0.21--------------------------------|     "
    - "L0.22[50,629] 1.01us 10mb    |----------------------L0.22-----------------------|                                  "
    - "L0.23[76,932] 1.01us 10mb      |-----------------------------------L0.23-----------------------------------|       "
    - "L0.24[42,986] 1.01us 10mb   |---------------------------------------L0.24---------------------------------------|  "
    - "L0.25[173,950] 1.01us 10mb               |-------------------------------L0.25--------------------------------|     "
    - "L0.26[50,629] 1.01us 10mb    |----------------------L0.26-----------------------|                                  "
    - "L0.27[76,932] 1.02us 10mb      |-----------------------------------L0.27-----------------------------------|       "
    - "L0.28[42,986] 1.02us 10mb   |---------------------------------------L0.28---------------------------------------|  "
    - "L0.29[173,950] 1.02us 10mb               |-------------------------------L0.29--------------------------------|     "
    - "L0.30[50,629] 1.02us 10mb    |----------------------L0.30-----------------------|                                  "
    - "L0.31[76,932] 1.02us 10mb      |-----------------------------------L0.31-----------------------------------|       "
    - "L0.32[42,986] 1.02us 10mb   |---------------------------------------L0.32---------------------------------------|  "
    - "L0.33[173,950] 1.02us 10mb               |-------------------------------L0.33--------------------------------|     "
    - "L0.34[50,629] 1.02us 10mb    |----------------------L0.34-----------------------|                                  "
    - "L0.35[76,932] 1.02us 10mb      |-----------------------------------L0.35-----------------------------------|       "
    - "L0.36[42,986] 1.02us 10mb   |---------------------------------------L0.36---------------------------------------|  "
    - "L0.37[173,950] 1.03us 10mb               |-------------------------------L0.37--------------------------------|     "
    - "L0.38[50,629] 1.03us 10mb    |----------------------L0.38-----------------------|                                  "
    - "L0.39[76,932] 1.03us 10mb      |-----------------------------------L0.39-----------------------------------|       "
    - "L0.40[42,986] 1.03us 10mb   |---------------------------------------L0.40---------------------------------------|  "
    - "L0.41[173,950] 1.03us 10mb               |-------------------------------L0.41--------------------------------|     "
    - "L0.42[50,629] 1.03us 10mb    |----------------------L0.42-----------------------|                                  "
    - "L0.43[76,932] 1.03us 10mb      |-----------------------------------L0.43-----------------------------------|       "
    - "L0.44[42,986] 1.03us 10mb   |---------------------------------------L0.44---------------------------------------|  "
    - "L0.45[173,950] 1.03us 10mb               |-------------------------------L0.45--------------------------------|     "
    - "L0.46[50,629] 1.03us 10mb    |----------------------L0.46-----------------------|                                  "
    - "L0.47[76,932] 1.04us 10mb      |-----------------------------------L0.47-----------------------------------|       "
    - "L0.48[42,986] 1.04us 10mb   |---------------------------------------L0.48---------------------------------------|  "
    - "L0.49[173,950] 1.04us 10mb               |-------------------------------L0.49--------------------------------|     "
    - "L0.50[50,629] 1.04us 10mb    |----------------------L0.50-----------------------|                                  "
    - "L0.51[76,932] 1.04us 10mb      |-----------------------------------L0.51-----------------------------------|       "
    - "L0.52[42,986] 1.04us 10mb   |---------------------------------------L0.52---------------------------------------|  "
    - "L0.53[173,950] 1.04us 10mb               |-------------------------------L0.53--------------------------------|     "
    - "L0.54[50,629] 1.04us 10mb    |----------------------L0.54-----------------------|                                  "
    - "L0.55[76,932] 1.04us 10mb      |-----------------------------------L0.55-----------------------------------|       "
    - "L0.56[42,986] 1.05us 10mb   |---------------------------------------L0.56---------------------------------------|  "
    - "L0.57[173,950] 1.05us 10mb               |-------------------------------L0.57--------------------------------|     "
    - "L0.58[50,629] 1.05us 10mb    |----------------------L0.58-----------------------|                                  "
    - "L0.59[76,932] 1.05us 10mb      |-----------------------------------L0.59-----------------------------------|       "
    - "L0.60[42,986] 1.05us 10mb   |---------------------------------------L0.60---------------------------------------|  "
    - "L2                                                                                                                 "
    - "L2.1[0,99] 99ns 100mb    |-L2.1-|                                                                                  "
    - "L2.2[100,199] 199ns 100mb         |-L2.2-|                                                                         "
    - "L2.3[200,299] 299ns 100mb                  |-L2.3-|                                                                "
    - "L2.4[300,399] 399ns 100mb                           |-L2.4-|                                                       "
    - "L2.5[400,499] 499ns 100mb                                    |-L2.5-|                                              "
    - "L2.6[500,599] 599ns 100mb                                             |-L2.6-|                                     "
    - "L2.7[600,699] 699ns 100mb                                                      |-L2.7-|                            "
    - "L2.8[700,799] 799ns 100mb                                                               |-L2.8-|                   "
    - "L2.9[800,899] 899ns 100mb                                                                        |-L2.9-|          "
    - "L2.10[900,999] 999ns 100mb                                                                                 |L2.10-| "
    - "**** Simulation run 0, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.11[76,932] 1us        |-----------------------------------------L0.11------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 4mb                                 |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1us 3mb                                                                  |----------L0.?-----------| "
    - "**** Simulation run 1, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.12[42,986] 1us        |-----------------------------------------L0.12------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 3mb                                  |-----------L0.?------------|                               "
    - "L0.?[671,986] 1us 3mb                                                               |------------L0.?------------| "
    - "**** Simulation run 2, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.13[173,950] 1us       |-----------------------------------------L0.13------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1us 2mb    |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1us 4mb                         |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1us 4mb                                                             |-------------L0.?-------------| "
    - "**** Simulation run 3, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.14[50,629] 1us        |-----------------------------------------L0.14------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1us 5mb     |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1us 5mb                                                   |------------------L0.?------------------| "
    - "**** Simulation run 4, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.15[76,932] 1us        |-----------------------------------------L0.15------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 4mb                                 |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1us 3mb                                                                  |----------L0.?-----------| "
    - "**** Simulation run 5, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.16[42,986] 1us        |-----------------------------------------L0.16------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1us 3mb     |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1us 3mb                                  |-----------L0.?------------|                               "
    - "L0.?[671,986] 1us 3mb                                                               |------------L0.?------------| "
    - "**** Simulation run 6, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.17[173,950] 1.01us    |-----------------------------------------L0.17------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 7, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.18[50,629] 1.01us     |-----------------------------------------L0.18------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 8, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.19[76,932] 1.01us     |-----------------------------------------L0.19------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.01us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 9, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.20[42,986] 1.01us     |-----------------------------------------L0.20------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.01us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 10, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.21[173,950] 1.01us    |-----------------------------------------L0.21------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 11, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.22[50,629] 1.01us     |-----------------------------------------L0.22------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 12, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.23[76,932] 1.01us     |-----------------------------------------L0.23------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.01us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 13, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.24[42,986] 1.01us     |-----------------------------------------L0.24------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.01us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.01us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.01us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 14, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.25[173,950] 1.01us    |-----------------------------------------L0.25------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.01us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.01us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.01us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 15, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.26[50,629] 1.01us     |-----------------------------------------L0.26------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.01us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.01us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 16, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.27[76,932] 1.02us     |-----------------------------------------L0.27------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 17, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.28[42,986] 1.02us     |-----------------------------------------L0.28------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 18, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.29[173,950] 1.02us    |-----------------------------------------L0.29------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.02us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.02us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.02us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 19, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.30[50,629] 1.02us     |-----------------------------------------L0.30------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.02us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.02us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 20, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.31[76,932] 1.02us     |-----------------------------------------L0.31------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 21, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.32[42,986] 1.02us     |-----------------------------------------L0.32------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 22, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.33[173,950] 1.02us    |-----------------------------------------L0.33------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.02us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.02us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.02us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 23, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.34[50,629] 1.02us     |-----------------------------------------L0.34------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.02us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.02us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 24, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.35[76,932] 1.02us     |-----------------------------------------L0.35------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.02us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 25, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.36[42,986] 1.02us     |-----------------------------------------L0.36------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.02us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.02us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.02us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 26, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.37[173,950] 1.03us    |-----------------------------------------L0.37------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 27, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.38[50,629] 1.03us     |-----------------------------------------L0.38------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 28, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.39[76,932] 1.03us     |-----------------------------------------L0.39------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.03us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 29, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.40[42,986] 1.03us     |-----------------------------------------L0.40------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.03us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 30, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.41[173,950] 1.03us    |-----------------------------------------L0.41------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 31, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.42[50,629] 1.03us     |-----------------------------------------L0.42------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 32, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.43[76,932] 1.03us     |-----------------------------------------L0.43------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.03us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 33, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.44[42,986] 1.03us     |-----------------------------------------L0.44------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.03us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.03us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.03us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 34, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.45[173,950] 1.03us    |-----------------------------------------L0.45------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.03us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.03us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.03us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 35, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.46[50,629] 1.03us     |-----------------------------------------L0.46------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.03us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.03us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 36, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.47[76,932] 1.04us     |-----------------------------------------L0.47------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 37, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.48[42,986] 1.04us     |-----------------------------------------L0.48------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.04us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 38, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.49[173,950] 1.04us    |-----------------------------------------L0.49------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.04us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.04us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.04us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 39, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.50[50,629] 1.04us     |-----------------------------------------L0.50------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.04us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.04us 5mb                                                |------------------L0.?------------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 40 files: L0.11, L0.12, L0.13, L0.14, L0.15, L0.16, L0.17, L0.18, L0.19, L0.20, L0.21, L0.22, L0.23, L0.24, L0.25, L0.26, L0.27, L0.28, L0.29, L0.30, L0.31, L0.32, L0.33, L0.34, L0.35, L0.36, L0.37, L0.38, L0.39, L0.40, L0.41, L0.42, L0.43, L0.44, L0.45, L0.46, L0.47, L0.48, L0.49, L0.50"
    - "  Creating 110 files"
    - "**** Simulation run 40, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.51[76,932] 1.04us     |-----------------------------------------L0.51------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 41, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.52[42,986] 1.04us     |-----------------------------------------L0.52------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.04us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 42, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.53[173,950] 1.04us    |-----------------------------------------L0.53------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.04us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.04us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.04us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 43, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.54[50,629] 1.04us     |-----------------------------------------L0.54------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.04us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.04us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 44, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.55[76,932] 1.04us     |-----------------------------------------L0.55------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.04us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.04us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.04us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 45, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.56[42,986] 1.05us     |-----------------------------------------L0.56------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.05us 3mb                                                            |------------L0.?------------| "
    - "**** Simulation run 46, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.57[173,950] 1.05us    |-----------------------------------------L0.57------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[173,356] 1.05us 2mb |-------L0.?--------|                                                                     "
    - "L0.?[357,670] 1.05us 4mb                      |---------------L0.?---------------|                                 "
    - "L0.?[671,950] 1.05us 4mb                                                          |-------------L0.?-------------| "
    - "**** Simulation run 47, type=split(VerticalSplit)(split_times=[356]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.58[50,629] 1.05us     |-----------------------------------------L0.58------------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[50,356] 1.05us 5mb  |--------------------L0.?---------------------|                                           "
    - "L0.?[357,629] 1.05us 5mb                                                |------------------L0.?------------------| "
    - "**** Simulation run 48, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.59[76,932] 1.05us     |-----------------------------------------L0.59------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[76,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 4mb                              |-------------L0.?-------------|                             "
    - "L0.?[671,932] 1.05us 3mb                                                               |----------L0.?-----------| "
    - "**** Simulation run 49, type=split(VerticalSplit)(split_times=[356, 670]). 1 Input Files, 10mb total:"
    - "L0, all files 10mb                                                                                                 "
    - "L0.60[42,986] 1.05us     |-----------------------------------------L0.60------------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 10mb total:"
    - "L0                                                                                                                 "
    - "L0.?[42,356] 1.05us 3mb  |-----------L0.?------------|                                                             "
    - "L0.?[357,670] 1.05us 3mb                               |-----------L0.?------------|                               "
    - "L0.?[671,986] 1.05us 3mb                                                            |------------L0.?------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.51, L0.52, L0.53, L0.54, L0.55, L0.56, L0.57, L0.58, L0.59, L0.60"
    - "  Creating 28 files"
    - "**** Simulation run 50, type=compact(ManySmallFiles). 10 Input Files, 35mb total:"
    - "L0                                                                                                                 "
    - "L0.61[76,356] 1us 3mb             |------------------------------------L0.61-------------------------------------| "
    - "L0.64[42,356] 1us 3mb    |-----------------------------------------L0.64------------------------------------------|"
    - "L0.67[173,356] 1us 2mb                                        |----------------------L0.67-----------------------| "
    - "L0.70[50,356] 1us 5mb      |----------------------------------------L0.70----------------------------------------| "
    - "L0.72[76,356] 1us 3mb             |------------------------------------L0.72-------------------------------------| "
    - "L0.75[42,356] 1us 3mb    |-----------------------------------------L0.75------------------------------------------|"
    - "L0.78[173,356] 1.01us 2mb                                     |----------------------L0.78-----------------------| "
    - "L0.81[50,356] 1.01us 5mb   |----------------------------------------L0.81----------------------------------------| "
    - "L0.83[76,356] 1.01us 3mb          |------------------------------------L0.83-------------------------------------| "
    - "L0.86[42,356] 1.01us 3mb |-----------------------------------------L0.86------------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 35mb total:"
    - "L0, all files 35mb                                                                                                 "
    - "L0.?[42,356] 1.01us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.61, L0.64, L0.67, L0.70, L0.72, L0.75, L0.78, L0.81, L0.83, L0.86"
    - "  Creating 1 files"
    - "**** Simulation run 51, type=compact(ManySmallFiles). 10 Input Files, 36mb total:"
    - "L0                                                                                                                 "
    - "L0.89[173,356] 1.01us 2mb                                     |----------------------L0.89-----------------------| "
    - "L0.92[50,356] 1.01us 5mb   |----------------------------------------L0.92----------------------------------------| "
    - "L0.94[76,356] 1.01us 3mb          |------------------------------------L0.94-------------------------------------| "
    - "L0.97[42,356] 1.01us 3mb |-----------------------------------------L0.97------------------------------------------|"
    - "L0.100[173,356] 1.01us 2mb                                     |----------------------L0.100----------------------| "
    - "L0.103[50,356] 1.01us 5mb  |---------------------------------------L0.103----------------------------------------| "
    - "L0.105[76,356] 1.02us 3mb         |------------------------------------L0.105------------------------------------| "
    - "L0.108[42,356] 1.02us 3mb|-----------------------------------------L0.108-----------------------------------------|"
    - "L0.111[173,356] 1.02us 2mb                                     |----------------------L0.111----------------------| "
    - "L0.114[50,356] 1.02us 5mb  |---------------------------------------L0.114----------------------------------------| "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 36mb total:"
    - "L0, all files 36mb                                                                                                 "
    - "L0.?[42,356] 1.02us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.89, L0.92, L0.94, L0.97, L0.100, L0.103, L0.105, L0.108, L0.111, L0.114"
    - "  Creating 1 files"
    - "**** Simulation run 52, type=compact(ManySmallFiles). 10 Input Files, 35mb total:"
    - "L0                                                                                                                 "
    - "L0.116[76,356] 1.02us 3mb         |------------------------------------L0.116------------------------------------| "
    - "L0.119[42,356] 1.02us 3mb|-----------------------------------------L0.119-----------------------------------------|"
    - "L0.122[173,356] 1.02us 2mb                                     |----------------------L0.122----------------------| "
    - "L0.125[50,356] 1.02us 5mb  |---------------------------------------L0.125----------------------------------------| "
    - "L0.127[76,356] 1.02us 3mb         |------------------------------------L0.127------------------------------------| "
    - "L0.130[42,356] 1.02us 3mb|-----------------------------------------L0.130-----------------------------------------|"
    - "L0.133[173,356] 1.03us 2mb                                     |----------------------L0.133----------------------| "
    - "L0.136[50,356] 1.03us 5mb  |---------------------------------------L0.136----------------------------------------| "
    - "L0.138[76,356] 1.03us 3mb         |------------------------------------L0.138------------------------------------| "
    - "L0.141[42,356] 1.03us 3mb|-----------------------------------------L0.141-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 35mb total:"
    - "L0, all files 35mb                                                                                                 "
    - "L0.?[42,356] 1.03us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.116, L0.119, L0.122, L0.125, L0.127, L0.130, L0.133, L0.136, L0.138, L0.141"
    - "  Creating 1 files"
    - "**** Simulation run 53, type=compact(ManySmallFiles). 10 Input Files, 36mb total:"
    - "L0                                                                                                                 "
    - "L0.144[173,356] 1.03us 2mb                                     |----------------------L0.144----------------------| "
    - "L0.147[50,356] 1.03us 5mb  |---------------------------------------L0.147----------------------------------------| "
    - "L0.149[76,356] 1.03us 3mb         |------------------------------------L0.149------------------------------------| "
    - "L0.152[42,356] 1.03us 3mb|-----------------------------------------L0.152-----------------------------------------|"
    - "L0.155[173,356] 1.03us 2mb                                     |----------------------L0.155----------------------| "
    - "L0.158[50,356] 1.03us 5mb  |---------------------------------------L0.158----------------------------------------| "
    - "L0.160[76,356] 1.04us 3mb         |------------------------------------L0.160------------------------------------| "
    - "L0.163[42,356] 1.04us 3mb|-----------------------------------------L0.163-----------------------------------------|"
    - "L0.166[173,356] 1.04us 2mb                                     |----------------------L0.166----------------------| "
    - "L0.169[50,356] 1.04us 5mb  |---------------------------------------L0.169----------------------------------------| "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 36mb total:"
    - "L0, all files 36mb                                                                                                 "
    - "L0.?[42,356] 1.04us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.144, L0.147, L0.149, L0.152, L0.155, L0.158, L0.160, L0.163, L0.166, L0.169"
    - "  Creating 1 files"
    - "**** Simulation run 54, type=compact(ManySmallFiles). 10 Input Files, 35mb total:"
    - "L0                                                                                                                 "
    - "L0.171[76,356] 1.04us 3mb         |------------------------------------L0.171------------------------------------| "
    - "L0.174[42,356] 1.04us 3mb|-----------------------------------------L0.174-----------------------------------------|"
    - "L0.177[173,356] 1.04us 2mb                                     |----------------------L0.177----------------------| "
    - "L0.180[50,356] 1.04us 5mb  |---------------------------------------L0.180----------------------------------------| "
    - "L0.182[76,356] 1.04us 3mb         |------------------------------------L0.182------------------------------------| "
    - "L0.185[42,356] 1.05us 3mb|-----------------------------------------L0.185-----------------------------------------|"
    - "L0.188[173,356] 1.05us 2mb                                     |----------------------L0.188----------------------| "
    - "L0.191[50,356] 1.05us 5mb  |---------------------------------------L0.191----------------------------------------| "
    - "L0.193[76,356] 1.05us 3mb         |------------------------------------L0.193------------------------------------| "
    - "L0.196[42,356] 1.05us 3mb|-----------------------------------------L0.196-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 35mb total:"
    - "L0, all files 35mb                                                                                                 "
    - "L0.?[42,356] 1.05us      |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.171, L0.174, L0.177, L0.180, L0.182, L0.185, L0.188, L0.191, L0.193, L0.196"
    - "  Creating 1 files"
    - "**** Simulation run 55, type=compact(ManySmallFiles). 10 Input Files, 38mb total:"
    - "L0                                                                                                                 "
    - "L0.62[357,670] 1us 4mb   |-----------------------------------------L0.62------------------------------------------|"
    - "L0.65[357,670] 1us 3mb   |-----------------------------------------L0.65------------------------------------------|"
    - "L0.68[357,670] 1us 4mb   |-----------------------------------------L0.68------------------------------------------|"
    - "L0.71[357,629] 1us 5mb   |-----------------------------------L0.71------------------------------------|            "
    - "L0.73[357,670] 1us 4mb   |-----------------------------------------L0.73------------------------------------------|"
    - "L0.76[357,670] 1us 3mb   |-----------------------------------------L0.76------------------------------------------|"
    - "L0.79[357,670] 1.01us 4mb|-----------------------------------------L0.79------------------------------------------|"
    - "L0.82[357,629] 1.01us 5mb|-----------------------------------L0.82------------------------------------|            "
    - "L0.84[357,670] 1.01us 4mb|-----------------------------------------L0.84------------------------------------------|"
    - "L0.87[357,670] 1.01us 3mb|-----------------------------------------L0.87------------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.?[357,670] 1.01us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.62, L0.65, L0.68, L0.71, L0.73, L0.76, L0.79, L0.82, L0.84, L0.87"
    - "  Creating 1 files"
    - "**** Simulation run 56, type=compact(ManySmallFiles). 10 Input Files, 40mb total:"
    - "L0                                                                                                                 "
    - "L0.90[357,670] 1.01us 4mb|-----------------------------------------L0.90------------------------------------------|"
    - "L0.93[357,629] 1.01us 5mb|-----------------------------------L0.93------------------------------------|            "
    - "L0.95[357,670] 1.01us 4mb|-----------------------------------------L0.95------------------------------------------|"
    - "L0.98[357,670] 1.01us 3mb|-----------------------------------------L0.98------------------------------------------|"
    - "L0.101[357,670] 1.01us 4mb|-----------------------------------------L0.101-----------------------------------------|"
    - "L0.104[357,629] 1.01us 5mb|-----------------------------------L0.104-----------------------------------|            "
    - "L0.106[357,670] 1.02us 4mb|-----------------------------------------L0.106-----------------------------------------|"
    - "L0.109[357,670] 1.02us 3mb|-----------------------------------------L0.109-----------------------------------------|"
    - "L0.112[357,670] 1.02us 4mb|-----------------------------------------L0.112-----------------------------------------|"
    - "L0.115[357,629] 1.02us 5mb|-----------------------------------L0.115-----------------------------------|            "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 40mb total:"
    - "L0, all files 40mb                                                                                                 "
    - "L0.?[357,670] 1.02us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.90, L0.93, L0.95, L0.98, L0.101, L0.104, L0.106, L0.109, L0.112, L0.115"
    - "  Creating 1 files"
    - "**** Simulation run 57, type=compact(ManySmallFiles). 10 Input Files, 38mb total:"
    - "L0                                                                                                                 "
    - "L0.117[357,670] 1.02us 4mb|-----------------------------------------L0.117-----------------------------------------|"
    - "L0.120[357,670] 1.02us 3mb|-----------------------------------------L0.120-----------------------------------------|"
    - "L0.123[357,670] 1.02us 4mb|-----------------------------------------L0.123-----------------------------------------|"
    - "L0.126[357,629] 1.02us 5mb|-----------------------------------L0.126-----------------------------------|            "
    - "L0.128[357,670] 1.02us 4mb|-----------------------------------------L0.128-----------------------------------------|"
    - "L0.131[357,670] 1.02us 3mb|-----------------------------------------L0.131-----------------------------------------|"
    - "L0.134[357,670] 1.03us 4mb|-----------------------------------------L0.134-----------------------------------------|"
    - "L0.137[357,629] 1.03us 5mb|-----------------------------------L0.137-----------------------------------|            "
    - "L0.139[357,670] 1.03us 4mb|-----------------------------------------L0.139-----------------------------------------|"
    - "L0.142[357,670] 1.03us 3mb|-----------------------------------------L0.142-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.?[357,670] 1.03us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.117, L0.120, L0.123, L0.126, L0.128, L0.131, L0.134, L0.137, L0.139, L0.142"
    - "  Creating 1 files"
    - "**** Simulation run 58, type=compact(ManySmallFiles). 10 Input Files, 40mb total:"
    - "L0                                                                                                                 "
    - "L0.145[357,670] 1.03us 4mb|-----------------------------------------L0.145-----------------------------------------|"
    - "L0.148[357,629] 1.03us 5mb|-----------------------------------L0.148-----------------------------------|            "
    - "L0.150[357,670] 1.03us 4mb|-----------------------------------------L0.150-----------------------------------------|"
    - "L0.153[357,670] 1.03us 3mb|-----------------------------------------L0.153-----------------------------------------|"
    - "L0.156[357,670] 1.03us 4mb|-----------------------------------------L0.156-----------------------------------------|"
    - "L0.159[357,629] 1.03us 5mb|-----------------------------------L0.159-----------------------------------|            "
    - "L0.161[357,670] 1.04us 4mb|-----------------------------------------L0.161-----------------------------------------|"
    - "L0.164[357,670] 1.04us 3mb|-----------------------------------------L0.164-----------------------------------------|"
    - "L0.167[357,670] 1.04us 4mb|-----------------------------------------L0.167-----------------------------------------|"
    - "L0.170[357,629] 1.04us 5mb|-----------------------------------L0.170-----------------------------------|            "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 40mb total:"
    - "L0, all files 40mb                                                                                                 "
    - "L0.?[357,670] 1.04us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.145, L0.148, L0.150, L0.153, L0.156, L0.159, L0.161, L0.164, L0.167, L0.170"
    - "  Creating 1 files"
    - "**** Simulation run 59, type=compact(ManySmallFiles). 10 Input Files, 38mb total:"
    - "L0                                                                                                                 "
    - "L0.172[357,670] 1.04us 4mb|-----------------------------------------L0.172-----------------------------------------|"
    - "L0.175[357,670] 1.04us 3mb|-----------------------------------------L0.175-----------------------------------------|"
    - "L0.178[357,670] 1.04us 4mb|-----------------------------------------L0.178-----------------------------------------|"
    - "L0.181[357,629] 1.04us 5mb|-----------------------------------L0.181-----------------------------------|            "
    - "L0.183[357,670] 1.04us 4mb|-----------------------------------------L0.183-----------------------------------------|"
    - "L0.186[357,670] 1.05us 3mb|-----------------------------------------L0.186-----------------------------------------|"
    - "L0.189[357,670] 1.05us 4mb|-----------------------------------------L0.189-----------------------------------------|"
    - "L0.192[357,629] 1.05us 5mb|-----------------------------------L0.192-----------------------------------|            "
    - "L0.194[357,670] 1.05us 4mb|-----------------------------------------L0.194-----------------------------------------|"
    - "L0.197[357,670] 1.05us 3mb|-----------------------------------------L0.197-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.?[357,670] 1.05us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.172, L0.175, L0.178, L0.181, L0.183, L0.186, L0.189, L0.192, L0.194, L0.197"
    - "  Creating 1 files"
    - "**** Simulation run 60, type=compact(ManySmallFiles). 10 Input Files, 33mb total:"
    - "L0                                                                                                                 "
    - "L0.63[671,932] 1us 3mb   |---------------------------------L0.63----------------------------------|                "
    - "L0.66[671,986] 1us 3mb   |-----------------------------------------L0.66------------------------------------------|"
    - "L0.69[671,950] 1us 4mb   |------------------------------------L0.69------------------------------------|           "
    - "L0.74[671,932] 1us 3mb   |---------------------------------L0.74----------------------------------|                "
    - "L0.77[671,986] 1us 3mb   |-----------------------------------------L0.77------------------------------------------|"
    - "L0.80[671,950] 1.01us 4mb|------------------------------------L0.80------------------------------------|           "
    - "L0.85[671,932] 1.01us 3mb|---------------------------------L0.85----------------------------------|                "
    - "L0.88[671,986] 1.01us 3mb|-----------------------------------------L0.88------------------------------------------|"
    - "L0.91[671,950] 1.01us 4mb|------------------------------------L0.91------------------------------------|           "
    - "L0.96[671,932] 1.01us 3mb|---------------------------------L0.96----------------------------------|                "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 33mb total:"
    - "L0, all files 33mb                                                                                                 "
    - "L0.?[671,986] 1.01us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.63, L0.66, L0.69, L0.74, L0.77, L0.80, L0.85, L0.88, L0.91, L0.96"
    - "  Creating 1 files"
    - "**** Simulation run 61, type=compact(ManySmallFiles). 10 Input Files, 33mb total:"
    - "L0                                                                                                                 "
    - "L0.99[671,986] 1.01us 3mb|-----------------------------------------L0.99------------------------------------------|"
    - "L0.102[671,950] 1.01us 4mb|-----------------------------------L0.102------------------------------------|           "
    - "L0.107[671,932] 1.02us 3mb|---------------------------------L0.107---------------------------------|                "
    - "L0.110[671,986] 1.02us 3mb|-----------------------------------------L0.110-----------------------------------------|"
    - "L0.113[671,950] 1.02us 4mb|-----------------------------------L0.113------------------------------------|           "
    - "L0.118[671,932] 1.02us 3mb|---------------------------------L0.118---------------------------------|                "
    - "L0.121[671,986] 1.02us 3mb|-----------------------------------------L0.121-----------------------------------------|"
    - "L0.124[671,950] 1.02us 4mb|-----------------------------------L0.124------------------------------------|           "
    - "L0.129[671,932] 1.02us 3mb|---------------------------------L0.129---------------------------------|                "
    - "L0.132[671,986] 1.02us 3mb|-----------------------------------------L0.132-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 33mb total:"
    - "L0, all files 33mb                                                                                                 "
    - "L0.?[671,986] 1.02us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.99, L0.102, L0.107, L0.110, L0.113, L0.118, L0.121, L0.124, L0.129, L0.132"
    - "  Creating 1 files"
    - "**** Simulation run 62, type=compact(ManySmallFiles). 10 Input Files, 34mb total:"
    - "L0                                                                                                                 "
    - "L0.135[671,950] 1.03us 4mb|-----------------------------------L0.135------------------------------------|           "
    - "L0.140[671,932] 1.03us 3mb|---------------------------------L0.140---------------------------------|                "
    - "L0.143[671,986] 1.03us 3mb|-----------------------------------------L0.143-----------------------------------------|"
    - "L0.146[671,950] 1.03us 4mb|-----------------------------------L0.146------------------------------------|           "
    - "L0.151[671,932] 1.03us 3mb|---------------------------------L0.151---------------------------------|                "
    - "L0.154[671,986] 1.03us 3mb|-----------------------------------------L0.154-----------------------------------------|"
    - "L0.157[671,950] 1.03us 4mb|-----------------------------------L0.157------------------------------------|           "
    - "L0.162[671,932] 1.04us 3mb|---------------------------------L0.162---------------------------------|                "
    - "L0.165[671,986] 1.04us 3mb|-----------------------------------------L0.165-----------------------------------------|"
    - "L0.168[671,950] 1.04us 4mb|-----------------------------------L0.168------------------------------------|           "
    - "**** 1 Output Files (parquet_file_id not yet assigned), 34mb total:"
    - "L0, all files 34mb                                                                                                 "
    - "L0.?[671,986] 1.04us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.135, L0.140, L0.143, L0.146, L0.151, L0.154, L0.157, L0.162, L0.165, L0.168"
    - "  Creating 1 files"
    - "**** Simulation run 63, type=compact(ManySmallFiles). 8 Input Files, 27mb total:"
    - "L0                                                                                                                 "
    - "L0.173[671,932] 1.04us 3mb|---------------------------------L0.173---------------------------------|                "
    - "L0.176[671,986] 1.04us 3mb|-----------------------------------------L0.176-----------------------------------------|"
    - "L0.179[671,950] 1.04us 4mb|-----------------------------------L0.179------------------------------------|           "
    - "L0.184[671,932] 1.04us 3mb|---------------------------------L0.184---------------------------------|                "
    - "L0.187[671,986] 1.05us 3mb|-----------------------------------------L0.187-----------------------------------------|"
    - "L0.190[671,950] 1.05us 4mb|-----------------------------------L0.190------------------------------------|           "
    - "L0.195[671,932] 1.05us 3mb|---------------------------------L0.195---------------------------------|                "
    - "L0.198[671,986] 1.05us 3mb|-----------------------------------------L0.198-----------------------------------------|"
    - "**** 1 Output Files (parquet_file_id not yet assigned), 27mb total:"
    - "L0, all files 27mb                                                                                                 "
    - "L0.?[671,986] 1.05us     |------------------------------------------L0.?------------------------------------------|"
    - "Committing partition 1:"
    - "  Soft Deleting 8 files: L0.173, L0.176, L0.179, L0.184, L0.187, L0.190, L0.195, L0.198"
    - "  Creating 1 files"
    - "**** Simulation run 64, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[368, 694]). 8 Input Files, 290mb total:"
    - "L0                                                                                                                 "
    - "L0.199[42,356] 1.01us 35mb|----------L0.199-----------|                                                             "
    - "L0.204[357,670] 1.01us 38mb                              |----------L0.204-----------|                               "
    - "L0.209[671,986] 1.01us 33mb                                                           |-----------L0.209-----------| "
    - "L0.200[42,356] 1.02us 36mb|----------L0.200-----------|                                                             "
    - "L0.205[357,670] 1.02us 40mb                              |----------L0.205-----------|                               "
    - "L0.210[671,986] 1.02us 33mb                                                           |-----------L0.210-----------| "
    - "L0.201[42,356] 1.03us 35mb|----------L0.201-----------|                                                             "
    - "L0.206[357,670] 1.03us 38mb                              |----------L0.206-----------|                               "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 290mb total:"
    - "L1                                                                                                                 "
    - "L1.?[42,368] 1.03us 100mb|------------L1.?-------------|                                                           "
    - "L1.?[369,694] 1.03us 100mb                               |------------L1.?------------|                             "
    - "L1.?[695,986] 1.03us 90mb                                                              |----------L1.?-----------| "
    - "Committing partition 1:"
    - "  Soft Deleting 8 files: L0.199, L0.200, L0.201, L0.204, L0.205, L0.206, L0.209, L0.210"
    - "  Creating 3 files"
    - "**** Simulation run 65, type=split(ReduceOverlap)(split_times=[694]). 1 Input Files, 34mb total:"
    - "L0, all files 34mb                                                                                                 "
    - "L0.211[671,986] 1.04us   |-----------------------------------------L0.211-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 34mb total:"
    - "L0                                                                                                                 "
    - "L0.?[671,694] 1.04us 2mb |L0.?|                                                                                    "
    - "L0.?[695,986] 1.04us 31mb      |--------------------------------------L0.?---------------------------------------| "
    - "**** Simulation run 66, type=split(ReduceOverlap)(split_times=[368]). 1 Input Files, 40mb total:"
    - "L0, all files 40mb                                                                                                 "
    - "L0.207[357,670] 1.04us   |-----------------------------------------L0.207-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 40mb total:"
    - "L0                                                                                                                 "
    - "L0.?[357,368] 1.04us 1mb |L0.?|                                                                                    "
    - "L0.?[369,670] 1.04us 39mb   |----------------------------------------L0.?----------------------------------------| "
    - "**** Simulation run 67, type=split(ReduceOverlap)(split_times=[694]). 1 Input Files, 27mb total:"
    - "L0, all files 27mb                                                                                                 "
    - "L0.212[671,986] 1.05us   |-----------------------------------------L0.212-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 27mb total:"
    - "L0                                                                                                                 "
    - "L0.?[671,694] 1.05us 2mb |L0.?|                                                                                    "
    - "L0.?[695,986] 1.05us 25mb      |--------------------------------------L0.?---------------------------------------| "
    - "**** Simulation run 68, type=split(ReduceOverlap)(split_times=[368]). 1 Input Files, 38mb total:"
    - "L0, all files 38mb                                                                                                 "
    - "L0.208[357,670] 1.05us   |-----------------------------------------L0.208-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 38mb total:"
    - "L0                                                                                                                 "
    - "L0.?[357,368] 1.05us 1mb |L0.?|                                                                                    "
    - "L0.?[369,670] 1.05us 37mb   |----------------------------------------L0.?----------------------------------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 4 files: L0.207, L0.208, L0.211, L0.212"
    - "  Creating 8 files"
    - "**** Simulation run 69, type=split(CompactAndSplitOutput(TotalSizeLessThanMaxCompactSize))(split_times=[259, 476, 693, 910]). 10 Input Files, 436mb total:"
    - "L0                                                                                                                 "
    - "L0.216[671,694] 1.04us 2mb                                                           |L0.216|                       "
    - "L0.217[695,986] 1.04us 31mb                                                              |---------L0.217----------| "
    - "L0.202[42,356] 1.04us 36mb|----------L0.202-----------|                                                             "
    - "L0.218[357,368] 1.04us 1mb                              |L0.218|                                                    "
    - "L0.219[369,670] 1.04us 39mb                               |----------L0.219----------|                               "
    - "L0.203[42,356] 1.05us 35mb|----------L0.203-----------|                                                             "
    - "L0.222[357,368] 1.05us 1mb                              |L0.222|                                                    "
    - "L1                                                                                                                 "
    - "L1.214[369,694] 1.03us 100mb                               |-----------L1.214-----------|                             "
    - "L1.215[695,986] 1.03us 90mb                                                              |---------L1.215----------| "
    - "L1.213[42,368] 1.03us 100mb|-----------L1.213------------|                                                           "
    - "**** 5 Output Files (parquet_file_id not yet assigned), 436mb total:"
    - "L1                                                                                                                 "
    - "L1.?[42,259] 1.05us 100mb|-------L1.?-------|                                                                      "
    - "L1.?[260,476] 1.05us 100mb                    |-------L1.?-------|                                                  "
    - "L1.?[477,693] 1.05us 100mb                                         |-------L1.?-------|                             "
    - "L1.?[694,910] 1.05us 100mb                                                              |-------L1.?-------|        "
    - "L1.?[911,986] 1.05us 37mb                                                                                  |L1.?-| "
    - "Committing partition 1:"
    - "  Soft Deleting 10 files: L0.202, L0.203, L1.213, L1.214, L1.215, L0.216, L0.217, L0.218, L0.219, L0.222"
    - "  Creating 5 files"
    - "**** Simulation run 70, type=split(ReduceOverlap)(split_times=[910]). 1 Input Files, 25mb total:"
    - "L0, all files 25mb                                                                                                 "
    - "L0.221[695,986] 1.05us   |-----------------------------------------L0.221-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 25mb total:"
    - "L0                                                                                                                 "
    - "L0.?[695,910] 1.05us 18mb|------------------------------L0.?------------------------------|                        "
    - "L0.?[911,986] 1.05us 6mb                                                                   |--------L0.?---------| "
    - "**** Simulation run 71, type=split(ReduceOverlap)(split_times=[693]). 1 Input Files, 2mb total:"
    - "L0, all files 2mb                                                                                                  "
    - "L0.220[671,694] 1.05us   |-----------------------------------------L0.220-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 2mb total:"
    - "L0                                                                                                                 "
    - "L0.?[671,693] 1.05us 2mb |----------------------------------------L0.?----------------------------------------|    "
    - "L0.?[694,694] 1.05us 86kb                                                                                          |L0.?|"
    - "**** Simulation run 72, type=split(ReduceOverlap)(split_times=[476]). 1 Input Files, 37mb total:"
    - "L0, all files 37mb                                                                                                 "
    - "L0.223[369,670] 1.05us   |-----------------------------------------L0.223-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 37mb total:"
    - "L0                                                                                                                 "
    - "L0.?[369,476] 1.05us 13mb|------------L0.?-------------|                                                           "
    - "L0.?[477,670] 1.05us 24mb                                |-------------------------L0.?--------------------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 3 files: L0.220, L0.221, L0.223"
    - "  Creating 6 files"
    - "**** Simulation run 73, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[442, 624]). 5 Input Files, 239mb total:"
    - "L0                                                                                                                 "
    - "L0.233[369,476] 1.05us 13mb                      |-------L0.233-------|                                              "
    - "L0.234[477,670] 1.05us 24mb                                             |----------------L0.234----------------|     "
    - "L0.231[671,693] 1.05us 2mb                                                                                     |L0.231|"
    - "L1                                                                                                                 "
    - "L1.225[260,476] 1.05us 100mb|------------------L1.225------------------|                                              "
    - "L1.226[477,693] 1.05us 100mb                                             |------------------L1.226------------------| "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 239mb total:"
    - "L1                                                                                                                 "
    - "L1.?[260,442] 1.05us 100mb|---------------L1.?----------------|                                                     "
    - "L1.?[443,624] 1.05us 100mb                                      |---------------L1.?----------------|               "
    - "L1.?[625,693] 1.05us 39mb                                                                           |----L1.?----| "
    - "Committing partition 1:"
    - "  Soft Deleting 5 files: L1.225, L1.226, L0.231, L0.233, L0.234"
    - "  Creating 3 files"
    - "**** Simulation run 74, type=split(CompactAndSplitOutput(TotalSizeLessThanMaxCompactSize))(split_times=[876]). 5 Input Files, 161mb total:"
    - "L0                                                                                                                 "
    - "L0.230[911,986] 1.05us 6mb                                                                  |-------L0.230--------| "
    - "L0.229[695,910] 1.05us 18mb|-----------------------------L0.229-----------------------------|                        "
    - "L0.232[694,694] 1.05us 86kb|L0.232|                                                                                  "
    - "L1                                                                                                                 "
    - "L1.228[911,986] 1.05us 37mb                                                                  |-------L1.228--------| "
    - "L1.227[694,910] 1.05us 100mb|-----------------------------L1.227-----------------------------|                        "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 161mb total:"
    - "L1                                                                                                                 "
    - "L1.?[694,876] 1.05us 100mb|-------------------------L1.?-------------------------|                                  "
    - "L1.?[877,986] 1.05us 61mb                                                        |-------------L1.?--------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 5 files: L1.227, L1.228, L0.229, L0.230, L0.232"
    - "  Creating 2 files"
    - "**** Simulation run 75, type=split(ReduceOverlap)(split_times=[499, 599]). 1 Input Files, 100mb total:"
    - "L1, all files 100mb                                                                                                "
    - "L1.236[443,624] 1.05us   |-----------------------------------------L1.236-----------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 100mb total:"
    - "L1                                                                                                                 "
    - "L1.?[443,499] 1.05us 31mb|----------L1.?-----------|                                                               "
    - "L1.?[500,599] 1.05us 55mb                            |---------------------L1.?----------------------|             "
    - "L1.?[600,624] 1.05us 14mb                                                                              |--L1.?---| "
    - "**** Simulation run 76, type=split(ReduceOverlap)(split_times=[299, 399]). 1 Input Files, 100mb total:"
    - "L1, all files 100mb                                                                                                "
    - "L1.235[260,442] 1.05us   |-----------------------------------------L1.235-----------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 100mb total:"
    - "L1                                                                                                                 "
    - "L1.?[260,299] 1.05us 21mb|------L1.?-------|                                                                       "
    - "L1.?[300,399] 1.05us 55mb                   |---------------------L1.?---------------------|                       "
    - "L1.?[400,442] 1.05us 24mb                                                                     |-------L1.?-------| "
    - "**** Simulation run 77, type=split(ReduceOverlap)(split_times=[99, 199]). 1 Input Files, 100mb total:"
    - "L1, all files 100mb                                                                                                "
    - "L1.224[42,259] 1.05us    |-----------------------------------------L1.224-----------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 100mb total:"
    - "L1                                                                                                                 "
    - "L1.?[42,99] 1.05us 26mb  |--------L1.?---------|                                                                   "
    - "L1.?[100,199] 1.05us 46mb                        |-----------------L1.?------------------|                         "
    - "L1.?[200,259] 1.05us 28mb                                                                 |---------L1.?---------| "
    - "**** Simulation run 78, type=split(ReduceOverlap)(split_times=[899]). 1 Input Files, 61mb total:"
    - "L1, all files 61mb                                                                                                 "
    - "L1.239[877,986] 1.05us   |-----------------------------------------L1.239-----------------------------------------|"
    - "**** 2 Output Files (parquet_file_id not yet assigned), 61mb total:"
    - "L1                                                                                                                 "
    - "L1.?[877,899] 1.05us 12mb|------L1.?------|                                                                        "
    - "L1.?[900,986] 1.05us 48mb                  |--------------------------------L1.?---------------------------------| "
    - "**** Simulation run 79, type=split(ReduceOverlap)(split_times=[699, 799]). 1 Input Files, 100mb total:"
    - "L1, all files 100mb                                                                                                "
    - "L1.238[694,876] 1.05us   |-----------------------------------------L1.238-----------------------------------------|"
    - "**** 3 Output Files (parquet_file_id not yet assigned), 100mb total:"
    - "L1                                                                                                                 "
    - "L1.?[694,699] 1.05us 3mb |L1.?|                                                                                    "
    - "L1.?[700,799] 1.05us 55mb  |---------------------L1.?---------------------|                                        "
    - "L1.?[800,876] 1.05us 43mb                                                    |---------------L1.?----------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 5 files: L1.224, L1.235, L1.236, L1.238, L1.239"
    - "  Creating 14 files"
    - "**** Simulation run 80, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[74, 148]). 4 Input Files, 272mb total:"
    - "L1                                                                                                                 "
    - "L1.246[42,99] 1.05us 26mb                  |--------L1.246---------|                                               "
    - "L1.247[100,199] 1.05us 46mb                                             |------------------L1.247------------------| "
    - "L2                                                                                                                 "
    - "L2.1[0,99] 99ns 100mb    |-------------------L2.1-------------------|                                              "
    - "L2.2[100,199] 199ns 100mb                                             |-------------------L2.2-------------------| "
    - "**** 3 Output Files (parquet_file_id not yet assigned), 272mb total:"
    - "L2                                                                                                                 "
    - "L2.?[0,74] 1.05us 101mb  |-------------L2.?--------------|                                                         "
    - "L2.?[75,148] 1.05us 100mb                                 |-------------L2.?--------------|                        "
    - "L2.?[149,199] 1.05us 71mb                                                                   |--------L2.?--------| "
    - "Committing partition 1:"
    - "  Soft Deleting 4 files: L2.1, L2.2, L1.246, L1.247"
    - "  Creating 3 files"
    - "**** Simulation run 81, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[267]). 3 Input Files, 150mb total:"
    - "L1                                                                                                                 "
    - "L1.248[200,259] 1.05us 28mb|----------------------L1.248-----------------------|                                     "
    - "L1.243[260,299] 1.05us 21mb                                                      |-------------L1.243--------------| "
    - "L2                                                                                                                 "
    - "L2.3[200,299] 299ns 100mb|-----------------------------------------L2.3------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 150mb total:"
    - "L2                                                                                                                 "
    - "L2.?[200,267] 1.05us 101mb|---------------------------L2.?---------------------------|                              "
    - "L2.?[268,299] 1.05us 48mb                                                             |-----------L2.?-----------| "
    - "Committing partition 1:"
    - "  Soft Deleting 3 files: L2.3, L1.243, L1.248"
    - "  Creating 2 files"
    - "**** Simulation run 82, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[365]). 2 Input Files, 155mb total:"
    - "L1                                                                                                                 "
    - "L1.244[300,399] 1.05us 55mb|----------------------------------------L1.244-----------------------------------------| "
    - "L2                                                                                                                 "
    - "L2.4[300,399] 399ns 100mb|-----------------------------------------L2.4------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 155mb total:"
    - "L2                                                                                                                 "
    - "L2.?[300,365] 1.05us 101mb|--------------------------L2.?---------------------------|                               "
    - "L2.?[366,399] 1.05us 53mb                                                           |-----------L2.?------------|  "
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L2.4, L1.244"
    - "  Creating 2 files"
    - "**** Simulation run 83, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[464]). 3 Input Files, 155mb total:"
    - "L1                                                                                                                 "
    - "L1.245[400,442] 1.05us 24mb|---------------L1.245---------------|                                                    "
    - "L1.240[443,499] 1.05us 31mb                                       |---------------------L1.240---------------------| "
    - "L2                                                                                                                 "
    - "L2.5[400,499] 499ns 100mb|-----------------------------------------L2.5------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 155mb total:"
    - "L2                                                                                                                 "
    - "L2.?[400,464] 1.05us 100mb|--------------------------L2.?--------------------------|                                "
    - "L2.?[465,499] 1.05us 55mb                                                           |------------L2.?------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 3 files: L2.5, L1.240, L1.245"
    - "  Creating 2 files"
    - "**** Simulation run 84, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[565]). 2 Input Files, 155mb total:"
    - "L1                                                                                                                 "
    - "L1.241[500,599] 1.05us 55mb|----------------------------------------L1.241-----------------------------------------| "
    - "L2                                                                                                                 "
    - "L2.6[500,599] 599ns 100mb|-----------------------------------------L2.6------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 155mb total:"
    - "L2                                                                                                                 "
    - "L2.?[500,565] 1.05us 101mb|--------------------------L2.?---------------------------|                               "
    - "L2.?[566,599] 1.05us 53mb                                                           |-----------L2.?------------|  "
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L2.6, L1.241"
    - "  Creating 2 files"
    - "**** Simulation run 85, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[664]). 4 Input Files, 156mb total:"
    - "L1                                                                                                                 "
    - "L1.242[600,624] 1.05us 14mb|------L1.242-------|                                                                     "
    - "L1.237[625,693] 1.05us 39mb                      |--------------------------L1.237---------------------------|       "
    - "L1.251[694,699] 1.05us 3mb                                                                                     |L1.251|"
    - "L2                                                                                                                 "
    - "L2.7[600,699] 699ns 100mb|-----------------------------------------L2.7------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 156mb total:"
    - "L2                                                                                                                 "
    - "L2.?[600,664] 1.05us 101mb|--------------------------L2.?--------------------------|                                "
    - "L2.?[665,699] 1.05us 55mb                                                           |------------L2.?------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 4 files: L2.7, L1.237, L1.242, L1.251"
    - "  Creating 2 files"
    - "**** Simulation run 86, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[765]). 2 Input Files, 155mb total:"
    - "L1                                                                                                                 "
    - "L1.252[700,799] 1.05us 55mb|----------------------------------------L1.252-----------------------------------------| "
    - "L2                                                                                                                 "
    - "L2.8[700,799] 799ns 100mb|-----------------------------------------L2.8------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 155mb total:"
    - "L2                                                                                                                 "
    - "L2.?[700,765] 1.05us 102mb|--------------------------L2.?---------------------------|                               "
    - "L2.?[766,799] 1.05us 53mb                                                           |-----------L2.?------------|  "
    - "Committing partition 1:"
    - "  Soft Deleting 2 files: L2.8, L1.252"
    - "  Creating 2 files"
    - "**** Simulation run 87, type=split(CompactAndSplitOutput(FoundSubsetLessThanMaxCompactSize))(split_times=[864]). 3 Input Files, 155mb total:"
    - "L1                                                                                                                 "
    - "L1.253[800,876] 1.05us 43mb|------------------------------L1.253-------------------------------|                     "
    - "L1.249[877,899] 1.05us 12mb                                                                      |------L1.249------|"
    - "L2                                                                                                                 "
    - "L2.9[800,899] 899ns 100mb|-----------------------------------------L2.9------------------------------------------| "
    - "**** 2 Output Files (parquet_file_id not yet assigned), 155mb total:"
    - "L2                                                                                                                 "
    - "L2.?[800,864] 1.05us 100mb|--------------------------L2.?--------------------------|                                "
    - "L2.?[865,899] 1.05us 55mb                                                           |------------L2.?------------| "
    - "Committing partition 1:"
    - "  Soft Deleting 3 files: L2.9, L1.249, L1.253"
    - "  Creating 2 files"
    - "**** Final Output Files (4.04gb written)"
    - "L1                                                                                                                 "
    - "L1.250[900,986] 1.05us 48mb                                                                                 |L1.250| "
    - "L2                                                                                                                 "
    - "L2.10[900,999] 999ns 100mb                                                                                 |L2.10-| "
    - "L2.254[0,74] 1.05us 101mb|L2.254|                                                                                  "
    - "L2.255[75,148] 1.05us 100mb      |L2.255|                                                                            "
    - "L2.256[149,199] 1.05us 71mb             |L2.256|                                                                     "
    - "L2.257[200,267] 1.05us 101mb                  |L2.257|                                                                "
    - "L2.258[268,299] 1.05us 48mb                        |L2.258|                                                          "
    - "L2.259[300,365] 1.05us 101mb                           |L2.259|                                                       "
    - "L2.260[366,399] 1.05us 53mb                                |L2.260|                                                  "
    - "L2.261[400,464] 1.05us 100mb                                    |L2.261|                                              "
    - "L2.262[465,499] 1.05us 55mb                                         |L2.262|                                         "
    - "L2.263[500,565] 1.05us 101mb                                             |L2.263|                                     "
    - "L2.264[566,599] 1.05us 53mb                                                  |L2.264|                                "
    - "L2.265[600,664] 1.05us 101mb                                                      |L2.265|                            "
    - "L2.266[665,699] 1.05us 55mb                                                           |L2.266|                       "
    - "L2.267[700,765] 1.05us 102mb                                                               |L2.267|                   "
    - "L2.268[766,799] 1.05us 53mb                                                                     |L2.268|             "
    - "L2.269[800,864] 1.05us 100mb                                                                        |L2.269|          "
    - "L2.270[865,899] 1.05us 55mb                                                                             |L2.270|     "
    "###
    );
}

// The files in this case are from a partition that was doing incredibly inefficient L0->L0 compactions.
// For ease of looking at the files in simulator output, I mapped all times into a small time range.
// Specifically, all times in the L0s were sorted in a list, then the files timestamps were replaced with the 1 relative
// index from that list.
// The result is that time deltas are not representative of what's in the catalog, but the overlaps are replicated with
// small numbers as timestamps that are easier for a person to look at.
#[tokio::test]
async fn actual_case_from_catalog_1() {
    test_helpers::maybe_start_logging();

    let setup = layout_setup_builder()
        .await
        .with_max_desired_file_size_bytes(MAX_DESIRED_FILE_SIZE)
        .with_max_num_files_per_plan(20)
        .with_suppress_run_output()
        .with_partition_timeout(Duration::from_secs(10))
        .build()
        .await;

    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(1)
                .with_max_time(2)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(478836)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(3)
                .with_max_time(4)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(474866)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(5)
                .with_max_time(7)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(454768)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(6)
                .with_max_time(49)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(277569)
                .with_max_l0_created_at(Time::from_timestamp_nanos(127)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(8)
                .with_max_time(25)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(473373)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(9)
                .with_max_time(54)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(93159)
                .with_max_l0_created_at(Time::from_timestamp_nanos(141)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(10)
                .with_max_time(20)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(90782)
                .with_max_l0_created_at(Time::from_timestamp_nanos(153)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(11)
                .with_max_time(30)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(67575)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(12)
                .with_max_time(14)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(88947)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(13)
                .with_max_time(47)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(70227)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(15)
                .with_max_time(51)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(77719)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(16)
                .with_max_time(55)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(80887)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(17)
                .with_max_time(56)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(89902)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(18)
                .with_max_time(65)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(165529)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(19)
                .with_max_time(68)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(135875)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(21)
                .with_max_time(23)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(73234)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(22)
                .with_max_time(24)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(41743)
                .with_max_l0_created_at(Time::from_timestamp_nanos(311)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(26)
                .with_max_time(29)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(42785)
                .with_max_l0_created_at(Time::from_timestamp_nanos(311)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(27)
                .with_max_time(52)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(452507)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(28)
                .with_max_time(32)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(60040)
                .with_max_l0_created_at(Time::from_timestamp_nanos(316)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(31)
                .with_max_time(34)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(60890)
                .with_max_l0_created_at(Time::from_timestamp_nanos(319)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(33)
                .with_max_time(36)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(69687)
                .with_max_l0_created_at(Time::from_timestamp_nanos(323)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(35)
                .with_max_time(38)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(59141)
                .with_max_l0_created_at(Time::from_timestamp_nanos(325)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(37)
                .with_max_time(40)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(67287)
                .with_max_l0_created_at(Time::from_timestamp_nanos(328)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(39)
                .with_max_time(42)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(53545)
                .with_max_l0_created_at(Time::from_timestamp_nanos(335)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(41)
                .with_max_time(44)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(66218)
                .with_max_l0_created_at(Time::from_timestamp_nanos(336)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(43)
                .with_max_time(46)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(96870)
                .with_max_l0_created_at(Time::from_timestamp_nanos(340)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(45)
                .with_max_time(48)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(162922)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(50)
                .with_max_time(71)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(741563)
                .with_max_l0_created_at(Time::from_timestamp_nanos(127)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(53)
                .with_max_time(62)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(452298)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(57)
                .with_max_time(61)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(116428)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(58)
                .with_max_time(70)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(469192)
                .with_max_l0_created_at(Time::from_timestamp_nanos(141)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(59)
                .with_max_time(93)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(118295625)
                .with_max_l0_created_at(Time::from_timestamp_nanos(153)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(60)
                .with_max_time(102)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(21750626)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(63)
                .with_max_time(75)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(595460)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(64)
                .with_max_time(74)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(188078)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(66)
                .with_max_time(67)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(236787)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(69)
                .with_max_time(73)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(7073)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(72)
                .with_max_time(117)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(271967233)
                .with_max_l0_created_at(Time::from_timestamp_nanos(127)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(76)
                .with_max_time(80)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(296649)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(77)
                .with_max_time(77)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6400)
                .with_max_l0_created_at(Time::from_timestamp_nanos(311)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(78)
                .with_max_time(79)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6673)
                .with_max_l0_created_at(Time::from_timestamp_nanos(316)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(81)
                .with_max_time(133)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(260832981)
                .with_max_l0_created_at(Time::from_timestamp_nanos(141)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(82)
                .with_max_time(83)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(108736)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(84)
                .with_max_time(89)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(137579)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(85)
                .with_max_time(86)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6639)
                .with_max_l0_created_at(Time::from_timestamp_nanos(319)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(87)
                .with_max_time(88)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(126187)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(90)
                .with_max_time(97)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(158579)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(91)
                .with_max_time(92)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(107298)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(94)
                .with_max_time(143)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(120508643)
                .with_max_l0_created_at(Time::from_timestamp_nanos(153)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(95)
                .with_max_time(96)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(196729)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(98)
                .with_max_time(111)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(110870)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(99)
                .with_max_time(109)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(93360)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(100)
                .with_max_time(104)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6561)
                .with_max_l0_created_at(Time::from_timestamp_nanos(325)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(101)
                .with_max_time(106)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(68025)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(103)
                .with_max_time(173)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(228950896)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(105)
                .with_max_time(112)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(77925)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(107)
                .with_max_time(108)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(8237)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(110)
                .with_max_time(110)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6400)
                .with_max_l0_created_at(Time::from_timestamp_nanos(328)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(113)
                .with_max_time(116)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(48975)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(114)
                .with_max_time(124)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(79883481)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(115)
                .with_max_time(123)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(109212)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(116)
                .with_max_time(119)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(95486)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(118)
                .with_max_time(120)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(93781)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(121)
                .with_max_time(122)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(7448)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(125)
                .with_max_time(137)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(100265729)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(126)
                .with_max_time(136)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(102711)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(128)
                .with_max_time(142)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(119202)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(129)
                .with_max_time(130)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(9027)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(131)
                .with_max_time(132)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(25565)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(134)
                .with_max_time(145)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(34519040)
                .with_max_l0_created_at(Time::from_timestamp_nanos(179)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(135)
                .with_max_time(144)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(764185)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(138)
                .with_max_time(158)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(71505278)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(139)
                .with_max_time(159)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(183141)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(140)
                .with_max_time(154)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6701)
                .with_max_l0_created_at(Time::from_timestamp_nanos(336)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(146)
                .with_max_time(155)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(65266955)
                .with_max_l0_created_at(Time::from_timestamp_nanos(179)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(147)
                .with_max_time(160)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(21649346)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(148)
                .with_max_time(150)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(55409)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(149)
                .with_max_time(157)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(74432)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(151)
                .with_max_time(152)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(23495)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(156)
                .with_max_time(160)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(55979589)
                .with_max_l0_created_at(Time::from_timestamp_nanos(179)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(161)
                .with_max_time(163)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(1061014)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(161)
                .with_max_time(171)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(46116292)
                .with_max_l0_created_at(Time::from_timestamp_nanos(179)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(162)
                .with_max_time(167)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(43064)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(164)
                .with_max_time(174)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(99408169)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(165)
                .with_max_time(170)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(50372)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(166)
                .with_max_time(168)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(14716604)
                .with_max_l0_created_at(Time::from_timestamp_nanos(172)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(169)
                .with_max_time(181)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(172039)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(175)
                .with_max_time(180)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(136666078)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(176)
                .with_max_time(178)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(189566)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(177)
                .with_max_time(182)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(47820008)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(183)
                .with_max_time(197)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(211523341)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(184)
                .with_max_time(196)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(159235)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(185)
                .with_max_time(195)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(14985821)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(186)
                .with_max_time(187)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(17799)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(188)
                .with_max_time(193)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(52964586)
                .with_max_l0_created_at(Time::from_timestamp_nanos(191)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(189)
                .with_max_time(190)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6420)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(192)
                .with_max_time(194)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(37185)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(198)
                .with_max_time(204)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(48661531)
                .with_max_l0_created_at(Time::from_timestamp_nanos(212)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(199)
                .with_max_time(206)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(104533)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(200)
                .with_max_time(207)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(115840212)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(201)
                .with_max_time(203)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(27386)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(202)
                .with_max_time(205)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6485)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(208)
                .with_max_time(214)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(63573570)
                .with_max_l0_created_at(Time::from_timestamp_nanos(224)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(209)
                .with_max_time(215)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(73119)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(210)
                .with_max_time(211)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(6626)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(213)
                .with_max_time(221)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(103699116)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(216)
                .with_max_time(226)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(160045)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(217)
                .with_max_time(220)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(47126)
                .with_max_l0_created_at(Time::from_timestamp_nanos(340)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(218)
                .with_max_time(219)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(7923)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(222)
                .with_max_time(225)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(116506120)
                .with_max_l0_created_at(Time::from_timestamp_nanos(239)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(223)
                .with_max_time(228)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(122528493)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(227)
                .with_max_time(234)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(42963)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(229)
                .with_max_time(242)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(132343737)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(230)
                .with_max_time(231)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(25526)
                .with_max_l0_created_at(Time::from_timestamp_nanos(340)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(232)
                .with_max_time(244)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(52114677)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(233)
                .with_max_time(237)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(80814)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(235)
                .with_max_time(241)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(84586)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(236)
                .with_max_time(238)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(31508)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(240)
                .with_max_time(243)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(31292)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(245)
                .with_max_time(255)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(169461420)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(246)
                .with_max_time(249)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(32436)
                .with_max_l0_created_at(Time::from_timestamp_nanos(258)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(247)
                .with_max_time(250)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(30783)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(248)
                .with_max_time(254)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(116968)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(251)
                .with_max_time(255)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(20831132)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(252)
                .with_max_time(268)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(47079)
                .with_max_l0_created_at(Time::from_timestamp_nanos(340)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(253)
                .with_max_time(267)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(8012)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(256)
                .with_max_time(259)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(24905052)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(256)
                .with_max_time(262)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(26916757)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(257)
                .with_max_time(263)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(114015)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(260)
                .with_max_time(270)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(184997646)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(261)
                .with_max_time(264)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(10024382)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(265)
                .with_max_time(270)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(11941889)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(266)
                .with_max_time(269)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(45048)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(271)
                .with_max_time(277)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(101521806)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(271)
                .with_max_time(275)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(28959050)
                .with_max_l0_created_at(Time::from_timestamp_nanos(273)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(272)
                .with_max_time(276)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(81663)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(274)
                .with_max_time(281)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(30344109)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(278)
                .with_max_time(283)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(125782713)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(279)
                .with_max_time(284)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(109926)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(280)
                .with_max_time(285)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(13486)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(282)
                .with_max_time(286)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(34930420)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(287)
                .with_max_time(298)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(179171551)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(288)
                .with_max_time(291)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(27704)
                .with_max_l0_created_at(Time::from_timestamp_nanos(296)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(289)
                .with_max_time(299)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(73478274)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(290)
                .with_max_time(292)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(16412)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(293)
                .with_max_time(339)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(99066230)
                .with_max_l0_created_at(Time::from_timestamp_nanos(342)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(294)
                .with_max_time(321)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(55188)
                .with_max_l0_created_at(Time::from_timestamp_nanos(328)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(295)
                .with_max_time(332)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(198938676)
                .with_max_l0_created_at(Time::from_timestamp_nanos(335)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(297)
                .with_max_time(313)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(244238124)
                .with_max_l0_created_at(Time::from_timestamp_nanos(311)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(300)
                .with_max_time(307)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(176463536)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(301)
                .with_max_time(302)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(17116)
                .with_max_l0_created_at(Time::from_timestamp_nanos(306)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(303)
                .with_max_time(304)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(9993)
                .with_max_l0_created_at(Time::from_timestamp_nanos(325)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(305)
                .with_max_time(317)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(229578231)
                .with_max_l0_created_at(Time::from_timestamp_nanos(316)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(308)
                .with_max_time(309)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(12831)
                .with_max_l0_created_at(Time::from_timestamp_nanos(319)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(310)
                .with_max_time(320)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(222546135)
                .with_max_l0_created_at(Time::from_timestamp_nanos(319)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(312)
                .with_max_time(314)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(25989)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(315)
                .with_max_time(324)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(224750727)
                .with_max_l0_created_at(Time::from_timestamp_nanos(323)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(318)
                .with_max_time(326)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(224562423)
                .with_max_l0_created_at(Time::from_timestamp_nanos(325)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(322)
                .with_max_time(329)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(223130462)
                .with_max_l0_created_at(Time::from_timestamp_nanos(328)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(327)
                .with_max_time(333)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(191981570)
                .with_max_l0_created_at(Time::from_timestamp_nanos(336)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(330)
                .with_max_time(338)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(242123981)
                .with_max_l0_created_at(Time::from_timestamp_nanos(340)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(331)
                .with_max_time(338)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(243511891)
                .with_max_l0_created_at(Time::from_timestamp_nanos(341)),
        )
        .await;
    setup
        .partition
        .create_parquet_file(
            parquet_builder()
                .with_min_time(334)
                .with_max_time(337)
                .with_compaction_level(CompactionLevel::Initial)
                .with_file_size_bytes(30538013)
                .with_max_l0_created_at(Time::from_timestamp_nanos(336)),
        )
        .await;

    insta::assert_yaml_snapshot!(
        run_layout_scenario(&setup).await,
        @r###"
    ---
    - "**** Input Files "
    - "L0                                                                                                                 "
    - "L0.1[1,2] 342ns 468kb    |L0.1|                                                                                    "
    - "L0.2[3,4] 342ns 464kb    |L0.2|                                                                                    "
    - "L0.3[5,7] 342ns 444kb     |L0.3|                                                                                   "
    - "L0.4[6,49] 127ns 271kb    |--L0.4---|                                                                              "
    - "L0.5[8,25] 342ns 462kb    |L0.5|                                                                                   "
    - "L0.6[9,54] 141ns 91kb      |--L0.6---|                                                                             "
    - "L0.7[10,20] 153ns 89kb     |L0.7|                                                                                  "
    - "L0.8[11,30] 172ns 66kb     |L0.8|                                                                                  "
    - "L0.9[12,14] 191ns 87kb     |L0.9|                                                                                  "
    - "L0.10[13,47] 212ns 69kb     |-L0.10-|                                                                              "
    - "L0.11[15,51] 224ns 76kb     |-L0.11-|                                                                              "
    - "L0.12[16,55] 239ns 79kb     |-L0.12--|                                                                             "
    - "L0.13[17,56] 258ns 88kb      |-L0.13--|                                                                            "
    - "L0.14[18,65] 273ns 162kb     |--L0.14---|                                                                          "
    - "L0.15[19,68] 296ns 133kb     |---L0.15---|                                                                         "
    - "L0.16[21,23] 306ns 72kb       |L0.16|                                                                              "
    - "L0.17[22,24] 311ns 41kb       |L0.17|                                                                              "
    - "L0.18[26,29] 311ns 42kb        |L0.18|                                                                             "
    - "L0.19[27,52] 342ns 442kb       |L0.19|                                                                             "
    - "L0.20[28,32] 316ns 59kb         |L0.20|                                                                            "
    - "L0.21[31,34] 319ns 59kb         |L0.21|                                                                            "
    - "L0.22[33,36] 323ns 68kb          |L0.22|                                                                           "
    - "L0.23[35,38] 325ns 58kb           |L0.23|                                                                          "
    - "L0.24[37,40] 328ns 66kb           |L0.24|                                                                          "
    - "L0.25[39,42] 335ns 52kb            |L0.25|                                                                         "
    - "L0.26[41,44] 336ns 65kb            |L0.26|                                                                         "
    - "L0.27[43,46] 340ns 95kb             |L0.27|                                                                        "
    - "L0.28[45,48] 341ns 159kb            |L0.28|                                                                        "
    - "L0.29[50,71] 127ns 724kb              |L0.29|                                                                      "
    - "L0.30[53,62] 342ns 442kb              |L0.30|                                                                      "
    - "L0.31[57,61] 341ns 114kb               |L0.31|                                                                     "
    - "L0.32[58,70] 141ns 458kb                |L0.32|                                                                    "
    - "L0.33[59,93] 153ns 113mb                |-L0.33-|                                                                  "
    - "L0.34[60,102] 172ns 21mb                |--L0.34--|                                                                "
    - "L0.35[63,75] 342ns 582kb                 |L0.35|                                                                   "
    - "L0.36[64,74] 341ns 184kb                 |L0.36|                                                                   "
    - "L0.37[66,67] 212ns 231kb                  |L0.37|                                                                  "
    - "L0.38[69,73] 306ns 7kb                     |L0.38|                                                                 "
    - "L0.39[72,117] 127ns 259mb                  |--L0.39--|                                                             "
    - "L0.40[76,80] 342ns 290kb                    |L0.40|                                                                "
    - "L0.41[77,77] 311ns 6kb                       |L0.41|                                                               "
    - "L0.42[78,79] 316ns 7kb                       |L0.42|                                                               "
    - "L0.43[81,133] 141ns 249mb                     |---L0.43---|                                                        "
    - "L0.44[82,83] 342ns 106kb                      |L0.44|                                                              "
    - "L0.45[84,89] 342ns 134kb                       |L0.45|                                                             "
    - "L0.46[85,86] 319ns 6kb                         |L0.46|                                                             "
    - "L0.47[87,88] 341ns 123kb                       |L0.47|                                                             "
    - "L0.48[90,97] 342ns 155kb                        |L0.48|                                                            "
    - "L0.49[91,92] 341ns 105kb                        |L0.49|                                                            "
    - "L0.50[94,143] 153ns 115mb                        |---L0.50---|                                                     "
    - "L0.51[95,96] 224ns 192kb                          |L0.51|                                                          "
    - "L0.52[98,111] 342ns 108kb                         |L0.52|                                                          "
    - "L0.53[99,109] 212ns 91kb                           |L0.53|                                                         "
    - "L0.54[100,104] 325ns 6kb                           |L0.54|                                                         "
    - "L0.55[101,106] 191ns 66kb                          |L0.55|                                                         "
    - "L0.56[103,173] 172ns 218mb                           |-----L0.56------|                                             "
    - "L0.57[105,112] 224ns 76kb                           |L0.57|                                                        "
    - "L0.58[107,108] 239ns 8kb                             |L0.58|                                                       "
    - "L0.59[110,110] 328ns 6kb                              |L0.59|                                                      "
    - "L0.60[113,116] 224ns 48kb                             |L0.60|                                                      "
    - "L0.61[114,124] 172ns 76mb                              |L0.61|                                                     "
    - "L0.62[115,123] 342ns 107kb                              |L0.62|                                                     "
    - "L0.63[116,119] 239ns 93kb                              |L0.63|                                                     "
    - "L0.64[118,120] 258ns 92kb                               |L0.64|                                                    "
    - "L0.65[121,122] 273ns 7kb                                |L0.65|                                                    "
    - "L0.66[125,137] 172ns 96mb                                 |L0.66|                                                  "
    - "L0.67[126,136] 342ns 100kb                                 |L0.67|                                                  "
    - "L0.68[128,142] 273ns 116kb                                 |L0.68|                                                  "
    - "L0.69[129,130] 258ns 9kb                                   |L0.69|                                                 "
    - "L0.70[131,132] 341ns 25kb                                  |L0.70|                                                 "
    - "L0.71[134,145] 179ns 33mb                                   |L0.71|                                                "
    - "L0.72[135,144] 191ns 746kb                                   |L0.72|                                                "
    - "L0.73[138,158] 172ns 68mb                                    |L0.73|                                               "
    - "L0.74[139,159] 342ns 179kb                                    |L0.74|                                               "
    - "L0.75[140,154] 336ns 7kb                                      |L0.75|                                              "
    - "L0.76[146,155] 179ns 62mb                                      |L0.76|                                             "
    - "L0.77[147,160] 191ns 21mb                                      |L0.77|                                             "
    - "L0.78[148,150] 273ns 54kb                                       |L0.78|                                            "
    - "L0.79[149,157] 296ns 73kb                                       |L0.79|                                            "
    - "L0.80[151,152] 224ns 23kb                                       |L0.80|                                            "
    - "L0.81[156,160] 179ns 53mb                                         |L0.81|                                          "
    - "L0.82[161,163] 191ns 1mb                                           |L0.82|                                         "
    - "L0.83[161,171] 179ns 44mb                                          |L0.83|                                         "
    - "L0.84[162,167] 342ns 42kb                                          |L0.84|                                         "
    - "L0.85[164,174] 191ns 95mb                                           |L0.85|                                        "
    - "L0.86[165,170] 296ns 49kb                                           |L0.86|                                        "
    - "L0.87[166,168] 172ns 14mb                                           |L0.87|                                        "
    - "L0.88[169,181] 342ns 168kb                                            |L0.88|                                       "
    - "L0.89[175,180] 191ns 130mb                                              |L0.89|                                     "
    - "L0.90[176,178] 273ns 185kb                                              |L0.90|                                     "
    - "L0.91[177,182] 212ns 46mb                                              |L0.91|                                     "
    - "L0.92[183,197] 212ns 202mb                                                |L0.92|                                   "
    - "L0.93[184,196] 342ns 156kb                                                |L0.93|                                   "
    - "L0.94[185,195] 224ns 14mb                                                |L0.94|                                   "
    - "L0.95[186,187] 273ns 17kb                                                 |L0.95|                                  "
    - "L0.96[188,193] 191ns 51mb                                                 |L0.96|                                  "
    - "L0.97[189,190] 341ns 6kb                                                   |L0.97|                                 "
    - "L0.98[192,194] 296ns 36kb                                                  |L0.98|                                 "
    - "L0.99[198,204] 212ns 46mb                                                    |L0.99|                               "
    - "L0.100[199,206] 342ns 102kb                                                    |L0.100|                              "
    - "L0.101[200,207] 224ns 110mb                                                    |L0.101|                              "
    - "L0.102[201,203] 296ns 27kb                                                     |L0.102|                             "
    - "L0.103[202,205] 341ns 6kb                                                     |L0.103|                             "
    - "L0.104[208,214] 224ns 61mb                                                       |L0.104|                           "
    - "L0.105[209,215] 342ns 71kb                                                       |L0.105|                           "
    - "L0.106[210,211] 239ns 6kb                                                       |L0.106|                           "
    - "L0.107[213,221] 239ns 99mb                                                        |L0.107|                          "
    - "L0.108[216,226] 342ns 156kb                                                         |L0.108|                         "
    - "L0.109[217,220] 340ns 46kb                                                         |L0.109|                         "
    - "L0.110[218,219] 341ns 8kb                                                         |L0.110|                         "
    - "L0.111[222,225] 239ns 111mb                                                          |L0.111|                        "
    - "L0.112[223,228] 258ns 117mb                                                           |L0.112|                       "
    - "L0.113[227,234] 342ns 42kb                                                            |L0.113|                      "
    - "L0.114[229,242] 258ns 126mb                                                            |L0.114|                      "
    - "L0.115[230,231] 340ns 25kb                                                            |L0.115|                      "
    - "L0.116[232,244] 273ns 50mb                                                             |L0.116|                     "
    - "L0.117[233,237] 273ns 79kb                                                             |L0.117|                     "
    - "L0.118[235,241] 342ns 83kb                                                              |L0.118|                    "
    - "L0.119[236,238] 296ns 31kb                                                              |L0.119|                    "
    - "L0.120[240,243] 306ns 31kb                                                               |L0.120|                   "
    - "L0.121[245,255] 273ns 162mb                                                                |L0.121|                  "
    - "L0.122[246,249] 258ns 32kb                                                                 |L0.122|                 "
    - "L0.123[247,250] 306ns 30kb                                                                 |L0.123|                 "
    - "L0.124[248,254] 342ns 114kb                                                                 |L0.124|                 "
    - "L0.125[251,255] 273ns 20mb                                                                  |L0.125|                "
    - "L0.126[252,268] 340ns 46kb                                                                  |L0.126|                "
    - "L0.127[253,267] 341ns 8kb                                                                   |L0.127|               "
    - "L0.128[256,259] 273ns 24mb                                                                   |L0.128|               "
    - "L0.129[256,262] 273ns 26mb                                                                   |L0.129|               "
    - "L0.130[257,263] 342ns 111kb                                                                    |L0.130|              "
    - "L0.131[260,270] 273ns 176mb                                                                    |L0.131|              "
    - "L0.132[261,264] 296ns 10mb                                                                     |L0.132|             "
    - "L0.133[265,270] 296ns 11mb                                                                      |L0.133|            "
    - "L0.134[266,269] 342ns 44kb                                                                      |L0.134|            "
    - "L0.135[271,277] 296ns 97mb                                                                       |L0.135|           "
    - "L0.136[271,275] 273ns 28mb                                                                       |L0.136|           "
    - "L0.137[272,276] 342ns 80kb                                                                        |L0.137|          "
    - "L0.138[274,281] 306ns 29mb                                                                        |L0.138|          "
    - "L0.139[278,283] 296ns 120mb                                                                         |L0.139|         "
    - "L0.140[279,284] 342ns 107kb                                                                          |L0.140|        "
    - "L0.141[280,285] 306ns 13kb                                                                          |L0.141|        "
    - "L0.142[282,286] 306ns 33mb                                                                          |L0.142|        "
    - "L0.143[287,298] 306ns 171mb                                                                            |L0.143|      "
    - "L0.144[288,291] 296ns 27kb                                                                            |L0.144|      "
    - "L0.145[289,299] 306ns 70mb                                                                            |L0.145|      "
    - "L0.146[290,292] 342ns 16kb                                                                            |L0.146|      "
    - "L0.147[293,339] 342ns 94mb                                                                             |--L0.147--| "
    - "L0.148[294,321] 328ns 54kb                                                                              |L0.148|    "
    - "L0.149[295,332] 335ns 190mb                                                                              |L0.149-|   "
    - "L0.150[297,313] 311ns 233mb                                                                              |L0.150|    "
    - "L0.151[300,307] 306ns 168mb                                                                               |L0.151|   "
    - "L0.152[301,302] 306ns 17kb                                                                               |L0.152|   "
    - "L0.153[303,304] 325ns 10kb                                                                                |L0.153|  "
    - "L0.154[305,317] 316ns 219mb                                                                                |L0.154|  "
    - "L0.155[308,309] 319ns 13kb                                                                                 |L0.155| "
    - "L0.156[310,320] 319ns 212mb                                                                                  |L0.156|"
    - "L0.157[312,314] 341ns 25kb                                                                                  |L0.157|"
    - "L0.158[315,324] 323ns 214mb                                                                                   |L0.158|"
    - "L0.159[318,326] 325ns 214mb                                                                                    |L0.159|"
    - "L0.160[322,329] 328ns 213mb                                                                                     |L0.160|"
    - "L0.161[327,333] 336ns 183mb                                                                                      |L0.161|"
    - "L0.162[330,338] 340ns 231mb                                                                                       |L0.162|"
    - "L0.163[331,338] 341ns 232mb                                                                                       |L0.163|"
    - "L0.164[334,337] 336ns 29mb                                                                                        |L0.164|"
    - "WARNING: file L0.39[72,117] 127ns 259mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.43[81,133] 141ns 249mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.56[103,173] 172ns 218mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.92[183,197] 212ns 202mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.121[245,255] 273ns 162mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.131[260,270] 273ns 176mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.143[287,298] 306ns 171mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.149[295,332] 335ns 190mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.150[297,313] 311ns 233mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.151[300,307] 306ns 168mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.154[305,317] 316ns 219mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.156[310,320] 319ns 212mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.158[315,324] 323ns 214mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.159[318,326] 325ns 214mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.160[322,329] 328ns 213mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.161[327,333] 336ns 183mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.162[330,338] 340ns 231mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L0.163[331,338] 341ns 232mb exceeds soft limit 100mb by more than 50%"
    - "**** Final Output Files (15.78gb written)"
    - "L2                                                                                                                 "
    - "L2.590[134,149] 342ns 202mb                                   |L2.590|                                               "
    - "L2.591[150,165] 342ns 218mb                                       |L2.591|                                           "
    - "L2.592[166,171] 342ns 118mb                                           |L2.592|                                       "
    - "L2.595[183,197] 342ns 267mb                                                |L2.595|                                  "
    - "L2.596[198,207] 342ns 157mb                                                    |L2.596|                              "
    - "L2.597[208,220] 342ns 147mb                                                       |L2.597|                           "
    - "L2.598[221,232] 342ns 270mb                                                          |L2.598|                        "
    - "L2.599[233,244] 342ns 147mb                                                             |L2.599|                     "
    - "L2.600[262,270] 342ns 184mb                                                                     |L2.600|             "
    - "L2.601[271,276] 342ns 117mb                                                                       |L2.601|           "
    - "L2.602[277,281] 342ns 109mb                                                                         |L2.602|         "
    - "L2.607[297,299] 342ns 141mb                                                                              |L2.607|    "
    - "L2.608[245,253] 342ns 139mb                                                                |L2.608|                  "
    - "L2.609[254,261] 342ns 105mb                                                                   |L2.609|               "
    - "L2.613[309,311] 342ns 101mb                                                                                  |L2.613|"
    - "L2.614[312,314] 342ns 181mb                                                                                  |L2.614|"
    - "L2.615[315,317] 342ns 214mb                                                                                   |L2.615|"
    - "L2.616[318,320] 342ns 222mb                                                                                    |L2.616|"
    - "L2.617[321,323] 342ns 146mb                                                                                     |L2.617|"
    - "L2.618[324,326] 342ns 254mb                                                                                      |L2.618|"
    - "L2.619[327,329] 342ns 197mb                                                                                      |L2.619|"
    - "L2.620[330,332] 342ns 228mb                                                                                       |L2.620|"
    - "L2.621[333,335] 342ns 199mb                                                                                        |L2.621|"
    - "L2.622[336,337] 342ns 156mb                                                                                         |L2.622|"
    - "L2.623[338,338] 342ns 124mb                                                                                         |L2.623|"
    - "L2.624[1,30] 342ns 103mb |L2.624|                                                                                  "
    - "L2.635[31,53] 342ns 101mb       |L2.635|                                                                           "
    - "L2.636[54,75] 342ns 96mb               |L2.636|                                                                    "
    - "L2.637[76,88] 342ns 64mb                    |L2.637|                                                               "
    - "L2.638[172,177] 342ns 109mb                                             |L2.638|                                     "
    - "L2.639[178,182] 342ns 109mb                                               |L2.639|                                   "
    - "L2.640[89,99] 342ns 104mb                       |L2.640|                                                           "
    - "L2.641[100,109] 342ns 94mb                          |L2.641|                                                        "
    - "L2.642[110,111] 342ns 31mb                             |L2.642|                                                     "
    - "L2.643[282,288] 342ns 100mb                                                                          |L2.643|        "
    - "L2.646[112,119] 342ns 116mb                             |L2.646|                                                     "
    - "L2.647[120,126] 342ns 99mb                               |L2.647|                                                   "
    - "L2.648[127,130] 342ns 83mb                                 |L2.648|                                                 "
    - "L2.649[131,132] 342ns 38mb                                  |L2.649|                                                "
    - "L2.650[133,133] 342ns 38mb                                   |L2.650|                                               "
    - "L2.651[289,295] 342ns 115mb                                                                            |L2.651|      "
    - "L2.652[296,296] 342ns 19mb                                                                              |L2.652|    "
    - "L2.653[300,303] 342ns 110mb                                                                               |L2.653|   "
    - "L2.656[304,306] 342ns 113mb                                                                                |L2.656|  "
    - "L2.657[307,308] 342ns 113mb                                                                                 |L2.657| "
    - "L2.658[339,339] 342ns 25mb                                                                                          |L2.658|"
    - "WARNING: file L2.590[134,149] 342ns 202mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.591[150,165] 342ns 218mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.595[183,197] 342ns 267mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.596[198,207] 342ns 157mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.598[221,232] 342ns 270mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.600[262,270] 342ns 184mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.614[312,314] 342ns 181mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.615[315,317] 342ns 214mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.616[318,320] 342ns 222mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.618[324,326] 342ns 254mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.619[327,329] 342ns 197mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.620[330,332] 342ns 228mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.621[333,335] 342ns 199mb exceeds soft limit 100mb by more than 50%"
    - "WARNING: file L2.622[336,337] 342ns 156mb exceeds soft limit 100mb by more than 50%"
    "###
    );
}
