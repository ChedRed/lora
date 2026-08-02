PlayerJumped = false
PlayertwoJumped = false

PlayerIt = true

function lora.load()
    lora.set.physics.gravity(0, -10)
    lora.set.physics.hertz(200)

    PlayerShape = lora.new.mesh({
        { 0.,   0.,   0., 0., 1., 0., 0., 1. },
        { -6.4, 25.6, 0., 0., 1., 0., 0., 1. },
        { 32.,  0.,   0., 0., 1., 0., 0., 1. },
        { 16,   40,   0., 0., 1., 0., 0., 1. },
        { 38.4, 25.6, 0., 0., 1., 0., 0., 1. },
    }, { 0, 1, 2, 3, 4 })
    PlayerCollider = lora.new.collider(PlayerShape, "dynamic")
    PlayerSpawner = lora.new.spawner(PlayerShape, PlayerCollider)
    PlayerObject = PlayerSpawner:spawn(200, 100, 0)

    -- PlayertwoShape = lora.new.image("resources/image.png", 1)
    PlayertwoShape = lora.new.mesh({
        { 0.,   0.,   0., 0., 0., 1., 0., 1. },
        { -6.4, 25.6, 0., 0., 0., 1., 0., 1. },
        { 32.,  0.,   0., 0., 0., 1., 0., 1. },
        { 16,   40,   0., 0., 0., 1., 0., 1. },
        { 38.4, 25.6, 0., 0., 0., 1., 0., 1. },
    }, { 0, 1, 2, 3, 4 })
    PlayertwoCollider = lora.new.collider(PlayertwoShape, "dynamic")
    PlayertwoSpawner = lora.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(2360, 100, 0)

    MapBorder = lora.new.border({
        { 0,    0 },
        { 2560, 0 },
        { 2560, 1600 },
        { 0,    1600 },
        { 0,    0 },
    })
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
        PlayerObject:set_position(200, 0)
        PlayerObject:set_motion(0, 0)
        PlayertwoObject:set_position(2360, 0)
        PlayertwoObject:set_motion(0, 0)
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
        PlayerObject:add_torque(1)
    end
    if lora.get.key.state("d") then
        PlayerObject:add_torque(-1)
    end

    if lora.get.key.state("j") then
        PlayertwoObject:add_torque(1)
    end
    if lora.get.key.state("l") then
        PlayertwoObject:add_torque(-1)
    end
end

function lora.render()
    local drawpos;
    if PlayerIt then
        drawpos = PlayerObject:world_center()
    else
        drawpos = PlayertwoObject:world_center()
    end
    lora.draw.circle(drawpos[1], drawpos[2] + 50, 10, { 1, 1, 1, 1 })
end