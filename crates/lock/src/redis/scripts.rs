pub(super) const ACQUIRE: &str = r#"
local epoch = string.match(redis.call('INFO','server'),'run_id:(%x+)')
if epoch ~= ARGV[1] then return {-2} end
local policy = redis.call('GET',KEYS[1])
if policy and policy ~= ARGV[2] then return {-3} end
local clock = redis.call('TIME')
local now = tonumber(clock[1])*1000+math.floor(tonumber(clock[2])/1000)
local expired = redis.call('ZRANGEBYSCORE',KEYS[2],'-inf',now)
for _,token in ipairs(expired) do redis.call('HDEL',KEYS[3],token) end
redis.call('ZREMRANGEBYSCORE',KEYS[2],'-inf',now)
local count = redis.call('ZCARD',KEYS[2])
if ARGV[2] == 'readers' then
  if ARGV[3] == 'exclusive' then
    if count > 0 then return {0} end
  else
    local modes = redis.call('HVALS',KEYS[3])
    for _,mode in ipairs(modes) do if mode == 'exclusive' then return {0} end end
  end
elseif count >= tonumber(ARGV[2]) then return {0} end
redis.call('SET',KEYS[1],ARGV[2])
redis.call('INCR',KEYS[4])
local sequence = redis.call('GET',KEYS[4])
local token = epoch..':'..sequence
redis.call('ZADD',KEYS[2],now+tonumber(ARGV[4]),token)
redis.call('HSET',KEYS[3],token,ARGV[3])
return {1,epoch,sequence}
"#;

pub(super) const RENEW: &str = r#"
local epoch = string.match(redis.call('INFO','server'),'run_id:(%x+)')
if epoch ~= ARGV[1] then return {-2} end
local clock = redis.call('TIME')
local now = tonumber(clock[1])*1000+math.floor(tonumber(clock[2])/1000)
local held = redis.call('ZSCORE',KEYS[2],ARGV[2])
if not held or tonumber(held) <= now then return {0} end
redis.call('ZADD',KEYS[2],now+tonumber(ARGV[3]),ARGV[2])
return {1}
"#;

pub(super) const RELEASE: &str = r#"
local epoch = string.match(redis.call('INFO','server'),'run_id:(%x+)')
if epoch ~= ARGV[1] then return {-2} end
local clock = redis.call('TIME')
local now = tonumber(clock[1])*1000+math.floor(tonumber(clock[2])/1000)
local held = redis.call('ZSCORE',KEYS[2],ARGV[2])
if not held or tonumber(held) <= now then return {0} end
redis.call('ZREM',KEYS[2],ARGV[2])
redis.call('HDEL',KEYS[3],ARGV[2])
return {1}
"#;
