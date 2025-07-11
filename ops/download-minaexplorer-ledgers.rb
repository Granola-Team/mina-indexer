#! /usr/bin/env -S ruby -w

# MinaExplorer maintains a set of staking ledger JSON dumps at
# https://storage.googleapis.com/mina-explorer-ledgers/
#
# The files are named after the hash: `<hash>.json`. For example:
# jxyJvcM4j4Mi2hNh6aLTB6wPChm5KtaaYGZBhTurWWBJ1278z5X.json
#
# The files can be fetched with, for example: 'curl', a web browser, or this
# script.
#
# See also: https://docs.minaexplorer.com/minaexplorer/data-archive

require "fileutils"
require "open-uri"

dest = ARGV[0]

ledgers = [
  ["25", "jxMFuoiz2pCzKib7NY1JDeR8Q1YxDSWzVMHGK5hTpXP4tuV8nh5"],
  ["24", "jwKbLH2f2xRAqgV1ohz618idSEmbARLJyjRUdnNFrsTwRVqVJCb"],
  ["23", "jwGzgaRhjAwZvfowL6LhqC6mHEZ7mCeXd6Ps4njfUQYAvYcRRoQ"],
  ["22", "jxyJvcM4j4Mi2hNh6aLTB6wPChm5KtaaYGZBhTurWWBJ1278z5X"],
  ["21", "jxFEmi9RUo7SArPwupC3QWdWPdoCYEWwhQhkMg9EeCM6QQV5HFs"],
  ["20", "jwNgkdZmaT8kekYmspUncrDSdT5HQqrJQNvWCJ6JcGeKATyW4BZ"],
  ["19", "jxyb5UaaPCLztB6KBujxyPMZj29d9NC5x6rfWPW8SU8pgpTVaJo"],
  ["18", "jxo6V3o7nMjK3arQd4Mm7Z4SZhErEELbb67ajMRPBnAmTSy6szy"],
  ["17", "jxNrphKDqCbNQTgvmZN3wvYynGTBjDbX4xnJMaNCiM3pkZK5cuh"],
  ["16", "jxDw3ESZbHAcPUCHumF11PaJXtEJcdcjxcUaGgETyi8XpoF6Smr"],
  ["15", "jwG7JJmejgi1zjWpgfPEyHb1zwyQdPQV8t1kF9KHNKzLQmXNh1F"],
  ["14", "jwWgXXSyoRsYUrVS4hzR8cVprSTnigr5n1RLAPiU3GYiZnphqbS"],
  ["13", "jw8GREh6xXFeidrmNCBPdhnxb7UySmnQAhxpCZFhzEgRs2cBYqN"],
  ["12", "jxmhjnXWt2gBHZQakZxstuSm3x5so4A6iRKcHW3DSrS3CnxtGZe"],
  ["11", "jwvTnX2F11AbcPwQDKdmGYSpEq97JjboGgeQzZMaKDwrk9WhMxB"],
  ["10", "jxELBJq1CZj414V8b37msRiVS6sEEHQyHRgeWMgjPyHuAmzUnnK"],
  ["9", "jxR8kF5S5ebAgcfFCYhJPer7LhYrCmgFGGH8pZnLfCo6BYcidrz"],
  ["8", "jwhzZrs2qUr1LrnWjgh5vToD9Fcbovu4rTWgFUJG1VqLpwB2NGh"],
  ["7", "jx2nVhtciYhqirG24xWCcDfyRjgxEbxahGbni2DhjkvaYq974DT"],
  ["6", "jxU9QkuXEZwKEYiwAJgbMzHQ1xDpYKhRua67kvnNhYZRA9jFPhe"],
  ["5", "jxq5Aw8Z4d7skbAmMFc3ZY8rvYSJ47NEs8Kv6ALfZRuqQ4WyXY4"],
  ["4", "jxMWSYe3RVgbMCgoc7ot5wx3QN3M8YXPTgKyBMwKwSCdaY89jvQ"],
  ["3", "jwPSjgLj5AsJtA1oTqMasQrxpeZx7pmGy5HKnAAhPBC4tYYvEj5"],
  ["2", "jw8dq1FtwJxbwqU1aYCxjY98fE21CqMDMynsXzRwAHAvM6yhx5A"],
  ["1", "jwgzfxD5rEnSP3k4UiZu2569FfhJ1SRUvabfTz21e4btwBHg3jq"],
  ["0", "jxsAidvKvEQJMC7Z2wkLrFGzCqUxpFMRhAj4K5o49eiFLhKSyXL"],
  ["79", "jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD"],
  ["78", "jxXwNfemxGwZcxKGhfrwzfE4QfxxGm5mkYieHQCafFkb6QBf9Xo"],
  ["77", "jwqNEHtM8gmFAThkBWzU2DQiUuK1rW52Z8zsHyxMtwxCMovLu5K"],
  ["76", "jwqkCmcBJDi7XVRuW3dJpTGJ8ZbFeWo1iuzbQPmt536GeC5YChN"],
  ["75", "jx25quMPEvvipny2VxwDys5yCHaUL8oCMapfLv4eoRrsxEKm4pD"],
  ["74", "jxvumaCvujr7UzW1qCB87YR2RWu8CqvkwrCmHY8kkwpvN4WbTJn"],
  ["73", "jwyody4XQNTnGxkXQEKf87AN27wXadAjYgnGLAtvHahDkn2uWDU"],
  ["72", "jx4YTukDZVaFoiwYpKzzPmoCNzZgyXG1nHQkN7mwoJoB8aXMAmt"],
  ["71", "jxiXyAr4NX6Ne1jxMU4WsiYc6SeBajSQZgmro9b63yDfQEeunD3"],
  ["70", "jxw6YYsPFbC7bPqCcc6pVShATXbebaX1cxFqeV7Kyo1Pa5L3TU4"],
  ["69", "jxQwGGbtjRnhT1k7CqyASPKihyjFdtYSnJMANxdyWbHvGUofn8t"],
  ["68", "jxJbw37Kd7KxNvy5yd322NFwYZUdsXCeeEfjqGJ3cY9ukMmxBiW"],
  ["67", "jxKCrryFrvzBE4iUURcS9zNTKcRdejiE9K28Bqcu7Us7RQqNfdL"],
  ["66", "jwvsYHPfACRUFYLL5NknBJc7zEY1q8t9rQfF8ek2pk2dUuKCz5J"],
  ["65", "jxAzD4eVVmY4bFF9QnMrEmjG8rEXEgVCFbD4H85LVZu4c4Zmi9D"],
  ["64", "jx4MPGB51t9MjrUh7NSsU6dLaouAb9bE2xu8b79kzmkEtKezwfw"],
  ["63", "jwbeXmeEZ2aYSyXnBbsMqWUxVDouwYZdzCqBejTaoecCkHoinPy"],
  ["62", "jy1jMBD7atiiheMxufJQDDfBuC2BjSXGj2HC5uSjXXsjAsGZt71"],
  ["61", "jxwahv5MsbGaUwSdAhyQA7Gr7atsyQbcju289PkoAnS4UgHGdce"],
  ["60", "jxKpSD4zcfKCSQQd3CG3yBqiesbUqm7eucRqLSvi9T1gUXtUPR5"],
  ["59", "jxffUAqcai9KoheQDcG46CCczjMRzFk61oXSokjaKvphicMpPj5"],
  ["58", "jwUe5igYAtQWZpcVYxt6xPJywnCZqDiNng9xZQLSZfZKpLZ61Hp"],
  ["57", "jwgDB316LgQr15vmZYC5G9gjcizT5MPssZbQkvtBLtqpi93mbMw"],
  ["56", "jxXZTgUtCJmJnuwURmNMhoJWQ44X1kRLaKXtuYRFxnT9GFGSnnj"],
  ["55", "jw8dXuUqXVgd6NvmpryGmFLnRv1176oozHAro8gMFwj8yuvhBeS"],
  ["54", "jx6taGcqX3HpWcz558wWNnJcne99jiQQiR7AnE7Ny8cQB1ASDVK"],
  ["53", "jxyqGrB5cSEavMbcMyNXhFMLcWpvbLR9a73GLqbTyPKVREkDjDM"],
  ["52", "jxWkqFVYsmQrXQZ2kkujynVj3TfbLfhVSgrY73CSVDpc17Bp3L6"],
  ["51", "jwuGkeeB2rxs2Cr679nZMVZpWms6QoEkcgt82Z2jsjB9X1MuJwW"],
  ["50", "jxaswvEn5WF82AHLwbzMJN5Ty6RNAH9azqMV2R9q4sJStCpMp3w"],
  ["49", "jwe5YREbjxzPCKe3eK7KfW5yXMdh71ca9mnMFfx9dBBQnRB6Rkb"],
  ["48", "jxZGkwwaAEXdKaFB12jdxfedApFQ4gDJ58aiSjNw9VUffBgAmdg"],
  ["47", "jwfyNt9AX6zRoWf67EcAzSQSDdLsS7Y8gZQPKmceCKo9C4hyKyX"],
  ["46", "jxQXzUkst2L9Ma9g9YQ3kfpgB5v5Znr1vrYb1mupakc5y7T89H8"],
  ["45", "jxWMPncjMY9VhwehhVKHhobvJuAhZcdx5kfUtX4V3dy9Rw9aMZA"],
  ["44", "jxdhN2AXg5v3c6KbGdmNW58hvdiTVULhXF3yztD8CdKNnGdf3jp"],
  ["43", "jxRhDLj6Q62jjRDNS2yYtDu6yHziPx6yLNXvPdgMfZaF3NFvJic"],
  ["42", "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"],
  ["41", "jwPQHxrJ94osTLCAiHYBuA6L4KGjkDV9t1A4mhdUoVEmbt2gxha"],
  ["40", "jxxaCK9mMbnpCR3D5TS55Tit8y36E9jN8ER1P6Xry8TyHPYp1CY"],
  ["39", "jw9FBsiQK5uJVGd8nr333vvctg3hPKf5kZUHf7f5bnUojWyNt3Z"],
  ["38", "jwHyH1qgW4iBRHEJEDo4yaxMW82VgNCLmQwHNzVKSxTapisydbo"],
  ["37", "jwpXcZgEcdvSswWPkaKYBcq1jfydzqitb87psbroyW6FSmjiSL8"],
  ["36", "jwJLfz7Pqfr3eRDFAMDw5TJ4Q3aD7ZZpP8YWKdWWU2iHU137NUE"],
  ["35", "jx1t9ivUkPJq9QfewYxFEc9GGLQVRZupDa9LRYFQeqpr9JPb1jj"],
  ["34", "jxHoMZnbhR25patdD3SeNQPe3U9MPctcRSRvPw7p7rpTKcZLB6t"],
  ["33", "jw9ZJUdzn6NYSinWYuSEV27qKE2ZFXyvpzgxD5ZzsbyWYpeqnR8"],
  ["32", "jwVvWi6GLeL6mz9jVFtD1HA7GNVuqe8tjFedisASfk8WVmdcfKE"],
  ["31", "jwcWudRBTNZuMd1Tcyjzpr71buwc9RNmT2Jip1efA9eWvXcZiKL"],
  ["30", "jxDP6iJZGfNixGBiVasAnYYm1Fk29qWP2MecJ4mAg676DK7sQCM"],
  ["29", "jxSi26fHMFyv8kxj4nBDDwi5FBt4oJummDjnfPodDbsNBzyjQdU"],
  ["28", "jx29wpTRDF8tuMFXgqT8inkJhb5chPjtZiwgTHzs6GxsvAy5KiH"],
  ["27", "jxsdc9d3AkKmVSWZQExucepfuLwfzQHtZpiCFArGqtfVe5jveiZ"],
  ["26", "jxAqNQwwU31ez8JPg6aJxugdX4uYnKFwbWGjqRtAxkfBBsLf2gf"],
  ["25", "jx3Z9VyiCTMdif3cHZQVs1zfLKmkE8Z6N2CzTfDFi3gM6XyJaRa"],
  ["24", "jwHGGFvSf4BVMuQs65gXb384cGzdkbQDyr9rVUPnGDXa1kKJNne"],
  ["23", "jwb5g4nyyMFvrXqN9wZLLb2TUx3Ux4gJ5F1k8Rt5nT9Eyaw9mZK"],
  ["22", "jwV7BsK9rBf5uRWqMZmWKVAUcEcd7pDAo9NCFTrvSvXRjHCwypF"],
  ["21", "jxix1ap5gwXmiiwRqjijDv5KbHmnjAfj19CDywRLT1J8yTADcsT"],
  ["20", "jxBBSjakhQRKLbUM7z99KXNnMke2GbdcJyqpD9gyRoJJybsMRqh"],
  ["19", "jxos2foijoacWtcKdjzqwv2PrU7je8XFDnsSVNGgrgJaJLeA8VE"],
  ["18", "jx5PU6GmyUqNuCnHNRF3pjHp7CTXXiCog4zJ1WcwHdyF3EJJ1Px"],
  ["17", "jwe63YTTUcc2b4sFdP54ehCZ3Dp9sZKshwCmtoVP3bidzfPfcxw"],
  ["16", "jwAXd4GZgxE3YCwqs99g4MpLNiEV2ZfZPstyah4jxo753AVgL6R"],
  ["15", "jxn15ATGoe4WGgYpbssxJH9XW8NXRDy22WvSsBqvMqcnLPgPAwN"],
  ["14", "jxPj7F7aRew1zvpW9JaGSgt9xmJitenrRSM6YGKnuhe5HXqyZtZ"],
  ["13", "jwq7sAxDuN9MrdLjAQULoyrY5hWa6g52SVq8EmajBeBY38zamgz"],
  ["12", "jx4itrnmDkG3ptAiwhitJHt9K8stgFFoenrkZrm2prbtaS54xQU"],
  ["11", "jx2XUFjvsvtTKB4HPAzih5boAtuoR34kxjEoU1RUhfXTATyx8tw"],
  ["10", "jxhjiLBeMR7pgtV8ogcJvqXdr6asoNrC3g6hoUzEDLBSnZoxUDJ"],
  ["9", "jxVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk"],
  ["8", "jxct9rteQ7wjhQVf7h4mGQmGZprJMkjbzEWgU7VvV6HEq2DN5yA"],
  ["7", "jxQgtuyHp8nA2P6F9CSRrLcVeHi8Ap7wVHeNnH2UbSX15izcSHK"],
  ["6", "jwJXdYzAikMHnTsw2kYyS1WQxJrGQsy1FKT5c18eHP2wGANafKf"],
  ["5", "jxVF5YbC3B5Rk6ibfsL97WaqojfxrgWtEqMJST9pb4X8s3kRD2T"],
  ["4", "jwPwVsSPZ2tmmGbp8UrWGmDgFDrrzTPpYcjpWosckmcVZV2kcW7"],
  ["3", "jxRySSfk8kJZVj46zveaToDUJUC2GtprmeK7poqWymEzB6d2Tun"],
  ["2", "jwAAZcXndLYxb8w4LTU2d4K1qT3dL8Ck2jKzVEf9t9GAyXweQRG"],
  ["1", "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"],
  ["0", "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"]
]

FileUtils.mkdir_p(dest)
unless Dir.exist?(dest)
  abort("#{dest} is not a directory")
end

ledgers.each do |row|
  hash = row[1]
  url = "https://storage.googleapis.com/mina-explorer-ledgers/#{hash}.json"
  puts "Fetching #{url}"
  content = URI.open(url).read # standard:disable Security/Open
  epoch = row[0]
  target = "#{dest}/mainnet-#{epoch}-#{hash}.json"
  File.write(target, content)
end
