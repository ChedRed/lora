PlayerJumped = false
PlayertwoJumped = false

PlayerIt = true

function lora.load()
    lora.set.physics.gravity(0, -10)
    lora.set.physics.hertz(200)

    -- PlayerShape = lora.new.mesh({
    --     { 0.,   0.,   0., 0., 1., 0., 0., 1. },
    --     { -6.4, 25.6, 0., 0., 1., 0., 0., 1. },
    --     { 32.,  0.,   0., 0., 1., 0., 0., 1. },
    --     { 16,   40,   0., 0., 1., 0., 0., 1. },
    --     { 38.4, 25.6, 0., 0., 1., 0., 0., 1. },
    -- }, { 0, 1, 2, 3, 4 })
    PlayerShape = lora.new.shape("rectangle", 48, 48, {1, 0, 0, 0})
    PlayerCollider = lora.new.collider(PlayerShape, "dynamic")
    PlayerSpawner = lora.new.spawner(PlayerShape, PlayerCollider)
    PlayerObject = PlayerSpawner:spawn(200, 100, 0)

    -- PlayertwoShape = lora.new.mesh({
    --     { 0.,   0.,   0., 0., 0., 1., 0., 1. },
    --     { -6.4, 25.6, 0., 0., 0., 1., 0., 1. },
    --     { 32.,  0.,   0., 0., 0., 1., 0., 1. },
    --     { 16,   40,   0., 0., 0., 1., 0., 1. },
    --     { 38.4, 25.6, 0., 0., 0., 1., 0., 1. },
    -- }, { 0, 1, 2, 3, 4 })
    PlayertwoShape = lora.new.shape("rectangle", 48, 48, {0, 1, 0, 0})
    PlayertwoCollider = lora.new.collider(PlayertwoShape, "dynamic")
    PlayertwoSpawner = lora.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(2360, 100, 0)

    MapBorder = lora.new.border({
        { 0,    0 },
        { 3024, 0 },
        { 3024, 1964 },
        { 0,    1964 },
        { 0,    0 },
    })
end

function lora.resized(x, y)
    lora.set.camera.position(x, y)
end

function lora.keypressed(key)
    if key == "w" then
        if not PlayerJumped then
            PlayerObject:impulse(0, 5)
            PlayerJumped = true
        end
    end
    if key == "i" then
        if not PlayertwoJumped then
            PlayertwoObject:impulse(0, 5)
            PlayertwoJumped = true
        end
    end
    if key == "r" then
        PlayerObject:position(200, 0)
        PlayerObject:motion(0, 0)
        PlayertwoObject:position(2360, 0)
        PlayertwoObject:motion(0, 0)
        PlayerIt = true
    end
end

function lora.keyreleased(key)
    if key == "w" then
        PlayerJumped = false
    end
    if key == "i" then
        PlayertwoJumped = false
    end
end

function lora.collision(one, two)
    PlayerUUID = PlayerObject:id()
    PlayertwoUUID = PlayertwoObject:id()

    if one == PlayerUUID or one == PlayertwoUUID then
        if two == PlayerUUID or two == PlayertwoUUID then
            PlayerIt = not PlayerIt
        end
    end
end

function lora.update(delta)
    if lora.get.key.state("a") then
        PlayerObject:torque(2)
    end
    if lora.get.key.state("d") then
        PlayerObject:torque(-2)

    end

    if lora.get.key.state("j") then
        PlayertwoObject:torque(2)
    end
    if lora.get.key.state("l") then
        PlayertwoObject:torque(-2)
    end
end

function lora.render()
    local drawpos;
    if PlayerIt then
        drawpos = PlayerObject:center()
    else
        drawpos = PlayertwoObject:center()
    end
    lora.draw.circle(drawpos[1], drawpos[2] + 50, 10, { 1, 1, 1, 1 })

    lora.draw.line(0, 0, 3024, 0, 1, {0, 0, 1, 1})
    lora.draw.line(3024, 0, 3024, 1964, 1, {0, 0, 1, 1})
    lora.draw.line(3024, 1964, 0, 1964, 1, {0, 0, 1, 1})
    lora.draw.line(0, 1964, 0, 0, 1, {0, 0, 1, 1})
end
