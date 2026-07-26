PlayerJumped = false
PlayertwoJumped = false

function lori.load()
    lori.set.window.title("Lori Test Application")

    lori.set.physics.gravity(0, -1000)
    lori.set.physics.hertz(200)

    PlayerShape = lori.new.mesh({
        { 0.,   0.,   0., 0., 1., 0., 0., 1. },
        { -6.4, 25.6, 0., 0., 1., 0., 0., 1. },
        { 32.,  0.,   0., 0., 1., 0., 0., 1. },
        { 16,   40,   0., 0., 1., 0., 0., 1. },
        { 38.4, 25.6, 0., 0., 1., 0., 0., 1. },
    }, { 0, 1, 2, 3, 4 })
    PlayerCollider = lori.new.collider(PlayerShape, "dynamic")
    PlayerSpawner = lori.new.spawner(PlayerShape, PlayerCollider)
    PlayerObject = PlayerSpawner:spawn(-20, 0, 0)

    PlayertwoShape = lori.new.shape("rectangle", 32, 32, { 0, 1, 0, 1 })
    PlayertwoCollider = lori.new.collider(PlayertwoShape, "diaxial")
    PlayertwoSpawner = lori.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(10, 0, 0)

    MapShape = lori.new.shape("rectangle", 2000, 10, { 0, 0, 1, 1 })
    MapCollider = lori.new.collider(MapShape, "static")
    MapSpawner = lori.new.spawner(MapShape, MapCollider)
    MapObject = MapSpawner:spawn(-500, -500, 0)
end

function lori.keypressed(key)
    if key == "w" then
        if not PlayerJumped then
            PlayerObject:impulse(0, 500)
            PlayerJumped = true
        end
    end
    if key == "i" then
        if not PlayertwoJumped then
            PlayertwoObject:impulse(0, 500)
            PlayertwoJumped = true
        end
    end
end

function lori.keyreleased(key)
    if key == "w" then
        PlayerJumped = false
    end
    if key == "i" then
        PlayertwoJumped = false
    end
end

function lori.update(delta)
    if lori.get.key.state("a") then
        PlayerObject:add_torque(10000)
    end
    if lori.get.key.state("d") then
        PlayerObject:add_torque(-10000)
    end

    if lori.get.key.state("j") then
        PlayertwoObject:add_force(-1000, 0)
    end
    if lori.get.key.state("l") then
        PlayertwoObject:add_force(1000, 0)
    end
end