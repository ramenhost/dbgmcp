/* A simulation of a double pendulum in C */

#include <stdio.h>
#include <math.h>
#include <string.h>
#include <unistd.h>
#include <stdlib.h>

/* Constants setting the screen size and frames per second */
const int screen_width = 80;
const int screen_height = 20;
const float x_scale = 2; // Sets aspect ratio to 1:1, since characters are twice as high as they are wide
const int image_buffer_size = screen_width * screen_height;
const int time_per_frame = 1000; // in microseconds
const int steps_per_frame = 100; // the time between frames is time_per_frame * steps_per_frame


struct Pendulum {
	int hook_x;
	int hook_y;
	float gravity;
	float dampening;
	float length1;
	float length2;
	float mass1;
	float mass2;
	float angle1;
	float angle2;
	float x1;
	float y1;
	float x2;
	float y2;
	float angular_velocity1;
	float angular_velocity2;
};

struct Pendulum make_pendulum(){
	struct Pendulum pendulum;

	// Set start values
	pendulum.hook_x = screen_width/4;
	pendulum.hook_y = screen_height/3;
	pendulum.gravity = 9.81;
	pendulum.dampening = 1.0;
	pendulum.length1 = 8;
	pendulum.length2 = 6;
	pendulum.mass1 = 4;
	pendulum.mass2 = 4;
	pendulum.angle1 = 3*M_PI/4;
	pendulum.angle2 = 2*M_PI/4;
	pendulum.angular_velocity1 = 0;
	pendulum.angular_velocity2 = 0;

	// Calculate positions
	pendulum.x1 = pendulum.hook_x + pendulum.length1 * sin(pendulum.angle1);
	pendulum.y1 = pendulum.hook_y + pendulum.length1 * cos(pendulum.angle1);
	pendulum.x2 = pendulum.x1 + pendulum.length2 * sin(pendulum.angle2);
	pendulum.y2 = pendulum.y1 + pendulum.length2 * cos(pendulum.angle2);

	return pendulum;
}

struct Pendulum update(struct Pendulum pendulum){
	// Calculate the new angles
	pendulum.angle1 += pendulum.angular_velocity1*time_per_frame/1000000;
	pendulum.angle2 += pendulum.angular_velocity2*time_per_frame/1000000;

	// Calculate the acceleration
	float g = pendulum.gravity;
	float m1 = pendulum.mass1;
	float m2 = pendulum.mass2;
	float a1 = pendulum.angle1;
	float a2 = pendulum.angle2;
	float av1 = pendulum.angular_velocity1;
	float av2 = pendulum.angular_velocity2;
	float l1 = pendulum.length1;
	float l2 = pendulum.length2;
	float acc1 = (-g*(2*m1+m2)*sin(a1) - m2*g*sin(a1-2*a2) - 2*sin(a1-a2)*m2*(av2*av2*l2 + av1*av1*l1*cos(a1-a2))) / (l1*(2*m1+m2-m2*cos(2*a1-2*a2)));
	float acc2 = (2*sin(a1-a2)*(av1*av1*l1*(m1+m2) + g*(m1+m2)*cos(a1) + av2*av2*l2*m2*cos(a1-a2))) / (l2*(2*m1+m2-m2*cos(2*a1-2*a2)));

	// Calculate the new angular velocities
	pendulum.angular_velocity1 *= pendulum.dampening;
	pendulum.angular_velocity2 *= pendulum.dampening;
	pendulum.angular_velocity1 += acc1*time_per_frame/1000000;
	pendulum.angular_velocity2 += acc2*time_per_frame/1000000;

	// Calculate the new positions
	pendulum.x1 = pendulum.hook_x + pendulum.length1 * sin(pendulum.angle1);
	pendulum.y1 = pendulum.hook_y + pendulum.length1 * cos(pendulum.angle1);
	pendulum.x2 = pendulum.x1 + pendulum.length2 * sin(pendulum.angle2);
	pendulum.y2 = pendulum.y1 + pendulum.length2 * cos(pendulum.angle2);

	return pendulum;
}


// A global buffer for the image.
char *buffer = NULL;

// Return the image buffer, making sure it's allocated
char * get_image_buffer(){
	if(buffer[0] == NULL){
		buffer = malloc(image_buffer_size*sizeof(char));
	}
	return malloc(image_buffer_size*sizeof(char));
}

void draw_pixel(char *buffer, int x, int y, char c){
	if(x >= 0 && x < screen_width || y >= 0 && y < screen_height){
		buffer[x + y * screen_width] = c;
	}
}

void draw(struct Pendulum pendulum){
	char *buffer = get_image_buffer();

	// Clear the screen
	printf("\033[2J");

	// Calculate on screen positions
	int hx = (int)(x_scale*pendulum.hook_x);
	int hy = pendulum.hook_y;
	int x1 = (int)(x_scale*pendulum.x1);
	int y1 = pendulum.y1;
	int x2 = (int)(x_scale*pendulum.x2);
	int y2 = pendulum.y2;

	// Empty the image buffer
	buffer = malloc(image_buffer_size*sizeof(char));
	for(int y=0; y<screen_height; y++){
		for (int x = 0; x < screen_width; x++){
			buffer[x + y * screen_width] = ' ';
		}
	}

	// Draw the hook and
	draw_pixel(buffer, hx, hy, 'x');
	draw_pixel(buffer, x1, y1, 'o');
	draw_pixel(buffer, x2, y2, 'o');

	// print the buffer
	for (int y = 0; y < screen_height; y++){
		for (int x = 0; x < screen_width; x++){
			putchar(buffer[x + y * screen_width]);
		}
		putchar('\n');
	}
}


int main(){
	struct Pendulum pendulum = make_pendulum();

	usleep(2000000);
	// Main loop
	while(1){
		for(int step = 0; step < steps_per_frame; step++){
			pendulum = update(pendulum);
		}
		draw(pendulum);
		usleep(steps_per_frame*time_per_frame);
	}
}


