#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

# Downloads Mina staking ledger logs from the Granola store.
# ARGV[0] = destination directory

DEST = ARGV[0]

require "fileutils"

clone_exe = "#{__dir__}/granola-rclone.rb"

FileUtils.mkdir_p(DEST)

# Fetch the ledgers list, if not already present.
#
ledgers_list = "#{DEST}/../staking-ledgers.list"
unless File.exist?(ledgers_list)
  warn "#{ledgers_list} does not exist. Fetching..."
  cmd = "#{clone_exe} lsf linode:granola-mina-staking-ledgers"
  warn "download-staking-ledgers issuing: #{cmd}"
  contents = `#{cmd}` || abort("Failure: #{cmd}")
  new_list = contents.lines(chomp: true).sort! do |a, b|
    a_split = a.split("-")
    b_split = b.split("-")
    a_num = a_split[1].to_i
    b_num = b_split[1].to_i
    if a_num < b_num
      -1
    elsif a_num > b_num
      1
    else
      a_split[2] <=> b_split[2]
    end
  end
  File.write(ledgers_list, new_list.join("\n"))
end

# Check to see if they're already present, building the list to fetch.
#
list = File.readlines(ledgers_list, chomp: true)
fetch = list.drop_while do |f|
  File.exist?("#{DEST}/#{f}")
end

if fetch.empty?
  warn "All files already present in #{DEST}."
else
  File.write("ledgers-to-fetch.list", fetch.join("\n"))
  system(
    clone_exe,
    "sync",
    "linode:granola-mina-staking-ledgers",
    "--files-from-raw", "ledgers-to-fetch.list",
    DEST
  ) || abort("Files sync failed in download-mina-staking-ledgers.rb")
  File.delete("ledgers-to-fetch.list")

  # Files should be read-only.
  #
  Dir["#{DEST}/*"].each do |f|
    File.chmod(0o444, f)
  end
end
