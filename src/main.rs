fn main() {
    println!("Hello, world!");
}


struct Interceptor{

}

struct Position{
    x: usize,
    y: usize,
    z: usize
}

struct DetectedThreat{
    position: Position,
    thread_level: usize, //i think it's better because it's good
}

struct InterceptorMessage{
    interceptor_type:
    positon: Position,
    angle: angle,
    threats: Vec<Threat>,
    used_ammo: usize, // used_ammo or ammo_amount?
}

struct Orchestrator;

fn main{
    // lit dans un dossier tout ce qui est mis directement
    // décide de ce qui se passe
    // essaie de les overlap
    // il faut gérer les tics facilement
}
