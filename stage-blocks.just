# Define variables
HARDFORK_GENESIS_BLOCK_HEIGHT := "359605"
STAGE_BLOCKS := "ops/stage-blocks.rb"

# Stage pre-hardfork blocks
v1 height output_dir:
    "{{STAGE_BLOCKS}}" 2 "{{height}}" mainnet "{{output_dir}}"

# Stage a single pre-hardfork block
v1-single height output_dir:
    "{{STAGE_BLOCKS}}" "{{height}}" "{{height}}" mainnet "{{output_dir}}"

# Stage a range of pre-hardfork blocks
v1-range start end output_dir:
    "{{STAGE_BLOCKS}}" "{{start}}" "{{end}}" mainnet "{{output_dir}}"

# Check if height is valid for hardfork
check-height height:
    #!/usr/bin/env ruby
    height = {{height}}.to_i
    hardfork_height = {{HARDFORK_GENESIS_BLOCK_HEIGHT}}.to_i
    if height <= hardfork_height
      puts "Hardfork block heights should be >= #{hardfork_height}"
      exit 1
    end

# Stage post-hardfork blocks
v2 height output_dir: (check-height height)
    #!/usr/bin/env ruby
    start = {{HARDFORK_GENESIS_BLOCK_HEIGHT}}.to_i + 1
    system("{{STAGE_BLOCKS}}", start.to_s, "{{height}}", "mainnet", "{{output_dir}}")

# Stage a single post-hardfork block
v2-single height output_dir: (check-height height)
    "{{STAGE_BLOCKS}}" "{{height}}" "{{height}}" mainnet "{{output_dir}}"

# Stage a range of post-hardfork blocks
v2-range start end output_dir: (check-height start) (check-height end)
    "{{STAGE_BLOCKS}}" "{{start}}" "{{end}}" mainnet "{{output_dir}}"
