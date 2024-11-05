# Rakefile

root_dir = pwd

desc 'Build'
task :build do
  chdir 'rust'
  sh "cargo build"
  chdir root_dir
end

desc 'Test'
task :test do
  chdir 'rust'
  sh "cargo test"
  chdir root_dir
end

desc 'Ingest blocks'
task :ingest => [:build] do
  chdir 'rust'
  sh "cargo run --bin ingest_blocks"
  chdir root_dir
end

desc 'Clean up build artifacts'
task :clean do
  chdir 'rust'
  sh "cargo clean"
  chdir root_dir
end
