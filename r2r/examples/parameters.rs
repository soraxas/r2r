use futures::{executor::LocalPool, prelude::*, task::LocalSpawnExt};

// try to run like this
// cargo run --example parameters -- --ros-args -p key1:=[hello,world] -p key2:=5.5 -r __ns:=/demo -r __node:=my_node
// then run
// ros2 param get /demo/my_node key2 # (should return 5.5)
// ros2 param set /demo/my_node key2 false
// ros2 param get /demo/my_node key2 # (should return false)

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Ros version: {}", r2r::ROS_DISTRO);

    // set up executor
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    // set up ros node
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "to_be_replaced", "to_be_replaced")?;

    // if you only need to load a parameter once at startup, it can be done like this.
    // errors can be propigated with the ? operator and enhanced with the `thiserror` and `anyhow` crates.
    // we do not use the ? operator here because we want the program to continue, even if the value is not set.
    let serial_interface_path = node.get_parameter::<String>("serial_interface");
    match serial_interface_path {
        Ok(serial_interface) => println!("Serial interface: {serial_interface}"),
        Err(error) => println!("Failed to get name of serial interface: {error}"),
    }

    // you can also get parameters as optional types.
    // this will be None if the parameter is not set. If the parameter is set but to the wrong type, this will
    // will produce an error.
    let baud_rate: Option<i64> = node.get_parameter("baud_rate")?;

    // because the baud_rate is an optional type, we can use `unwrap_or` to provide a default value.
    let baud_rate = baud_rate.unwrap_or(115200);
    println!("Baud rate: {baud_rate}");

    // make a parameter handler (once per node).
    // the parameter handler is optional, only spawn one if you need it.
    let (paramater_handler, parameter_events) = node.make_parameter_handler()?;
    // run parameter handler on your executor.
    spawner.spawn_local(paramater_handler)?;

    // parameter event stream. just print them
    spawner.spawn_local(async move {
        parameter_events
            .for_each(|(param_name, param_val)| {
                println!("parameter event: {} is now {:?}", param_name, param_val);
                future::ready(())
            })
            .await
    })?;

    println!("node name: {}", node.name()?);
    println!("node fully qualified name: {}", node.fully_qualified_name()?);
    println!("node namespace: {}", node.namespace()?);

    // print all params every 5 seconds.
    let mut timer = node.create_wall_timer(std::time::Duration::from_secs(5))?;
    let params = node.params.clone();
    spawner.spawn_local(async move {
        loop {
            println!("node parameters");
            params.lock().unwrap().iter().for_each(|(k, v)| {
                println!("{} - {:?}", k, v.value);
            });
            let _elapsed = timer.tick().await.expect("could not tick");
        }
    })?;

    loop {
        node.spin_once(std::time::Duration::from_millis(100));
        pool.run_until_stalled();
    }
}
