#! /usr/bin/env -S ruby -w

# -*- mode: ruby -*-

CONFIG_FILE = "#{__dir__}/rclone.conf"

args = [
  "rclone",
  "-vv", # '--log-level', 'INFO',
  "--config", CONFIG_FILE,
  "--buffer-size=128Mi",
  "--log-level=INFO"
  *ARGV
]
warn "granola-rclone issuing: #{args}"
system(*args) || abort("rclone failed")
