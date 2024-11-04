task :default => :menu

working_dir = pwd

desc 'Print the menu of targets'
task :menu do
  sh 'rake', '-T'
end

desc 'Check for prerequisite software'
task :prereqs do
  chdir 'rust'
  sh 'cargo', '--version'
  sh 'cargo', 'nextest', '--version'
  sh 'cargo', 'audit', '--version'
  sh 'cargo', 'clippy', '--version'
  sh 'cargo', 'machete', '--version'
  sh 'jq', '--version'
  sh 'check-jsonschema', '--version'
end

desc 'Clean the directory tree of build artifacts'
task :clean do
  rm_f 'result'
  chdir 'rust'
  sh 'cargo', 'clean'
end

desc 'Build the Rust code'
task :build do
  chdir 'rust'
  sh 'cargo', 'build', '--offline', '--release'
end

desc 'Verify that the Rust code is correctly formatted'
task :check_format do
  old = chdir 'rust'
  sh 'cargo', 'fmt', '--all', '--check'
end

desc 'Perform Rust-based unit tests'
task :test_unit do
  chdir 'rust'
  sh 'cargo', 'nextest', 'run', '--release'
end

desc 'Lint all code'
task :lint => :check_format do
  formatted_flake = `nixfmt < flake.nix`
  flake = File.read('flake.nix')
  flake == formatted_flake || abort('rbb')
end

desc 'Deploy Indexer'
task :deploy, [:deploy_type, :build_type, :blocks_count] => [:build] do |t, args|
  chdir working_dir
  args.with_defaults(deploy_type: 'prod', build_type: 'nix', blocks_count: '5000')
  sh "ruby ops/ingest-blocks.rb #{args[:deploy_type]} #{args[:build_type]} #{args[:blocks_count]}"
end
