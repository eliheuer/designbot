// Simple design script - no main function, just drawing commands
canvas.fill(Color::rgb(255, 200, 100));
canvas.rect(100.0, 100.0, 400.0, 400.0);

canvas.fill(Color::rgb(100, 200, 255));
canvas.oval(150.0, 150.0, 300.0, 300.0);

canvas.stroke(Color::black());
canvas.stroke_width(5.0);
canvas.line(50.0, 50.0, 550.0, 550.0);
