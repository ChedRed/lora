PlayerJumped = false

function lora.load()
    lora.set.window.title("Lora Test Application")

    lora.set.physics.gravity(0, -1000)
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
    PlayerObject = PlayerSpawner:spawn(-20, 0, 0)

    PlayertwoShape = lora.new.image("resources/image.png", 0.5)
    PlayertwoCollider = lora.new.collider(PlayertwoShape, "dynamic")
    PlayertwoSpawner = lora.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(10, 0, 0)

    MapShape = lora.new.shape("rectangle", 2000, 10, { 0, 0, 1, 1 })
    MapCollider = lora.new.collider(MapShape, "static")
    MapSpawner = lora.new.spawner(MapShape, MapCollider)
    MapObject = MapSpawner:spawn(-500, -500, 0)
end

function lora.keypressed(key)
    if key == "w" then
        if not PlayerJumped then
            PlayerObject:impulse(0, 500)
            PlayerJumped = true
        end
    end
end

function lora.keyreleased(key)
    if key == "w" then
        PlayerJumped = false
    end
end

function lora.update(delta)
    if lora.get.key.state("a") then
        PlayerObject:add_torque(10000)
    end
    if lora.get.key.state("d") then
        PlayerObject:add_torque(-10000)
    end

    if lora.get.key.state("i") then
        PlayertwoObject:add_force(0, 5000)
    end
    if lora.get.key.state("j") then
        PlayertwoObject:add_force(-1000, 0)
    end
    if lora.get.key.state("l") then
        PlayertwoObject:add_force(1000, 0)
    end
end