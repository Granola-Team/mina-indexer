def granola_rclone(*args)
  config_file = "#{__dir__}/rclone.conf"
  default_access_key = "PBCXKO3DINPHOQL2C6L9"
  default_secret_key = "QAvfnJBU844ETudG8VC4clZDGH672J0I7aRZSO4O"
  access_key = ENV["LINODE_OBJ_ACCESS_KEY"] || default_access_key
  secret_key = ENV["LINODE_OBJ_SECRET_KEY"] || default_secret_key

  args.unshift(
    "rclone",
    "--config", config_file,
    "--buffer-size=128Mi",
    "--log-level=INFO",
    "--s3-access-key-id=#{access_key}",
    "--s3-secret-access-key=#{secret_key}"
  )
  warn "granola_rclone issuing: #{args}"
  system(*args)
end
