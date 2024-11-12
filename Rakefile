# Rakefile

root_dir = pwd

if ENV['FLOX_ENV'].nil?
  sh 'flox activate' do |ok, status|
    if !ok
      puts "Failed to activate Flox environment. Is Flox installed? https://flox.dev/docs/install-flox/"
      exit 1
    end
  end
end

# Make task list the default action
task :default do
  system 'rake --tasks'
end

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

desc 'Clean database'
task :clean_db do
  chdir 'rust'
  sh "rm mina.db"
  chdir root_dir
end

desc 'Clean up build artifacts'
task :clean do
  chdir 'rust'
  sh "cargo clean"
  sh "rm mina.db"
  chdir root_dir
end

desc "Format code"
task :format do
  chdir 'rust'
  sh 'cargo-fmt --all'
  chdir root_dir
end

desc "Lint code"
task :lint do
  chdir 'rust'
  sh 'cargo machete --fix'
  sh 'cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings'
  chdir root_dir
end

desc "Check code formatting and run Clippy"
task :check do
  chdir 'rust'
  sh 'cargo machete'
  sh 'cargo-fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings'
  chdir root_dir
end

desc "Start database service"
task :start_db_service do
  sh "pg_ctl -D ./postgres_data -l logfile start"
end

desc "Create database"
task :create_db do
  sh "createdb -U mina_indexer mina_indexer"
end

desc "Remove database"
task :remove_db do
  sh "lsof -t -i :9002 | xargs kill -9 || true "
  sh "rm -rf postgres"
  sh "rm -rf postgres_data"
end

desc "Checks readiness of code"
task :ready => [:lint, :format, :check, :test]
