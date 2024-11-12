WITH epoch_data AS (
  SELECT DISTINCT source, balance, target
  FROM staking_ledgers sl
  JOIN staking_epochs se ON sl.staking_epoch_id = se.id
  WHERE se.epoch = 79
),
delegate_stats AS (
  SELECT 
    target,
    COUNT(*) as delegator_count,
    SUM(balance) as stake
  FROM epoch_data
  WHERE balance IS NOT NULL
  GROUP BY target
),
total_stake AS (
  SELECT SUM(balance) as total
  FROM epoch_data
  WHERE balance IS NOT NULL
),
final_stats AS (
  SELECT 
    ed.source as key,
    ed.target as delegate,
    ed.balance,
    ds.stake,
    ds.delegator_count,
    ts.total,
    CASE 
      WHEN ts.total = 0 THEN 0 
      ELSE (ds.stake::DECIMAL / ts.total * 100)
    END as stake_percentage
  FROM epoch_data ed
  CROSS JOIN total_stake ts
  LEFT JOIN delegate_stats ds ON ed.source = ds.target
  WHERE ed.balance IS NOT NULL
)
SELECT
  key,
  CAST(balance AS STRING) as balance,
  COALESCE(CAST(stake AS STRING), '0') as stake,
  stake_percentage,
  delegate,
  COALESCE(delegator_count, 0) as delegators
FROM final_stats
ORDER BY stake_percentage DESC;