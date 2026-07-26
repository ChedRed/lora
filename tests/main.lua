
function lori.load()
    lori.set.window.title("Lori Test Application")

    lori.set.gravity(0, -1000)

    PlayerShape = lori.new.mesh({
        { 0., 0., 0., 0., 1., 0., 0., 1. },
        { -6.4, 25.6, 0., 0., 1., 0., 0., 1. },
        { 32., 0., 0., 0., 1., 0., 0., 1. },
        { 16, 40, 0., 0., 1., 0., 0., 1. },
        { 38.4, 25.6, 0., 0., 1., 0., 0., 1. },
    }, { 0, 1, 2, 3, 4 })
    PlayerCollider = lori.new.collider(PlayerShape, "dynamic")
    PlayerSpawner = lori.new.spawner(PlayerShape, PlayerCollider)
    PlayerObject = PlayerSpawner:spawn(0, 0, 0)

    PlayertwoShape = lori.new.shape("rectangle", 32, 32, { 0, 1, 0, 1 })
    PlayertwoCollider = lori.new.collider(PlayertwoShape, "diaxial")
    PlayertwoSpawner = lori.new.spawner(PlayertwoShape, PlayertwoCollider)
    PlayertwoObject = PlayertwoSpawner:spawn(10, 0, 0)

    MapShape = lori.new.shape("rectangle", 2000, 10, { 0, 0, 1, 1 })
    MapCollider = lori.new.collider(MapShape, "static")
    MapSpawner = lori.new.spawner(MapShape, MapCollider)
    MapObject = MapSpawner:spawn(-500, -500, 0)
end

function lori.keyreleased(key)
    if key == "e" then
        print(lori.get.camera.position()[1])
    end
end

function lori.update(delta)
    if lori.get.key.state("w") then
        PlayerObject:move(0, 20)
    end
    if lori.get.key.state("s") then
        PlayerObject:push(0, -10)
    end
    if lori.get.key.state("a") then
        PlayerObject:push(-1000, 0)
    end
    if lori.get.key.state("d") then
        PlayerObject:push(1000, 0)
    end

    if lori.get.key.state("i") then
        PlayertwoObject:move(0, 20)
    end
    if lori.get.key.state("k") then
        PlayertwoObject:push(0, -1000)
    end
    if lori.get.key.state("j") then
        PlayertwoObject:push(-1000, 0)
    end
    if lori.get.key.state("l") then
        PlayertwoObject:push(1000, 0)
    end
end