# Rakefile

root_dir = pwd

task :build do
  chdir 'rust'
  sh "cargo build"
  chdir root_dir
end

# Task to list all tasks
task :ingest => [:build] do
  chdir 'rust'
  sh "cargo run --bin ingest_blocks"
  chdir root_dir
end

task :clean do
  chdir 'rust'
  sh "cargo clean"
  chdir root_dir
end
