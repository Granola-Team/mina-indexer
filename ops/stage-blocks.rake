HARDFORK_GENESIS_BLOCK_HEIGHT = "359605"
STAGE_BLOCKS = "#{ENV["TOPLEVEL"]}/ops/stage-blocks.rb"

task stage_blocks: ["stage_blocks:list"]

namespace :stage_blocks do
  desc "List available tasks"
  task :list do
    run("rake -T stage_blocks")
  end

  desc "Stage pre-hardfork blocks"
  task :v1, [:height, :output_dir] do |_, args|
    height = args[:height] || abort("Height parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    run("#{STAGE_BLOCKS} 2 #{height} mainnet #{output_dir}")
  end

  desc "Stage a single pre-hardfork block"
  task :v1_single, [:height, :output_dir] do |_, args|
    height = args[:height] || abort("Height parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    run("#{STAGE_BLOCKS} #{height} #{height} mainnet #{output_dir}")
  end

  desc "Stage a range of pre-hardfork blocks"
  task :v1_range, [:start, :end, :output_dir] do |_, args|
    start = args[:start] || abort("Start parameter is required")
    end_height = args[:end] || abort("End parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    run("#{STAGE_BLOCKS} #{start} #{end_height} mainnet #{output_dir}")
  end

  # Helper method to check if height is valid for hardfork
  def check_height(height)
    height = height.to_i
    hardfork_height = HARDFORK_GENESIS_BLOCK_HEIGHT.to_i
    if height <= hardfork_height
      abort("Hardfork block heights should be >= #{hardfork_height}")
    end
  end

  desc "Stage post-hardfork blocks"
  task :v2, [:height, :output_dir] do |_, args|
    height = args[:height] || abort("Height parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    check_height(height)
    start = HARDFORK_GENESIS_BLOCK_HEIGHT.to_i + 1
    run("#{STAGE_BLOCKS} #{start} #{height} mainnet #{output_dir}")
  end

  desc "Stage a single post-hardfork block"
  task :v2_single, [:height, :output_dir] do |_, args|
    height = args[:height] || abort("Height parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    check_height(height)
    run("#{STAGE_BLOCKS} #{height} #{height} mainnet #{output_dir}")
  end

  desc "Stage a range of post-hardfork blocks"
  task :v2_range, [:start, :end, :output_dir] do |_, args|
    start = args[:start] || abort("Start parameter is required")
    end_height = args[:end] || abort("End parameter is required")
    output_dir = args[:output_dir] || abort("Output directory parameter is required")
    check_height(start)
    check_height(end_height)
    run("#{STAGE_BLOCKS} #{start} #{end_height} mainnet #{output_dir}")
  end
end
