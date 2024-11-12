WITH epoch_data AS (
    SELECT DISTINCT
        source,
        balance,
        target
    FROM staking_ledgers
    INNER JOIN staking_epochs ON staking_epoch_id = id
    WHERE epoch = 79
),

delegate_stats AS (
    SELECT
        target,
        COUNT(*) AS delegator_count,
        SUM(balance) AS stake
    FROM epoch_data
    WHERE balance IS NOT NULL
    GROUP BY target
),

total_stake AS (
    SELECT SUM(balance) AS total
    FROM epoch_data
    WHERE balance IS NOT NULL
),

final_stats AS (
    SELECT
        epoch_data.source AS public_key,
        epoch_data.target AS delegate,
        epoch_data.balance,
        delegate_stats.stake,
        delegate_stats.delegator_count,
        total_stake.total,
        CASE
            WHEN total = 0 THEN 0
            ELSE (delegate_stats.stake::DECIMAL / total * 100)
        END AS stake_percentage
    FROM epoch_data
    CROSS JOIN total_stake
    LEFT JOIN delegate_stats ON epoch_data.source = delegate_stats.target
    WHERE epoch_data.balance IS NOT NULL
)

SELECT
    public_key,
    stake_percentage,
    delegate,
    balance::STRING AS balance,
    COALESCE(stake::STRING, '0') AS stake,
    COALESCE(delegator_count, 0) AS delegators
FROM final_stats
ORDER BY stake_percentage DESC;
