package navigation;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.PriorityQueue;

import bc.*;

public class Route {
	
	private int w;
	private int h;
	private ArrayList<Integer> distances;
	
	public Route(ArrayList<ArrayList<Point>> terrain, int w, int h, Point end) {
		this.w = w;
		this.h = h;
		HashSet<Point> visited = new HashSet<>();
		
		// Initialize priority queue
		distances = new ArrayList<>(Collections.nCopies(w*h, Integer.MAX_VALUE));
		PriorityQueue<Point> queue = new PriorityQueue<>((Point a, Point b) -> {
			return distances.get(a.y*w + a.x) - distances.get(b.y*w + b.x);
		});
		distances.set(end.y*w + end.x, 0);
		for (int y = 0; y < h; y++) {
			for (int x = 0; x < w; x++) {
				queue.add(new Point(x, y));
			}
		}

		while (queue.size() > 0) {
			Point node = queue.poll();
			visited.add(node);
			
			for (Point next : terrain.get(node.y*w + node.x)) {
				if (visited.contains(next)) { continue; }
				int da = distances.get(node.y*w + node.x) + 1;
				int db = distances.get(next.y*w + next.x);
				
				if (da < db) {
					distances.set(next.y*w + next.x, da);
					queue.remove(next);
					queue.add(next);
				}
			}
		}
		assert(distances.get(end.y*w + end.x) == 0);
	}
	
	public Direction from(GameController gc, MapLocation start) {
		int min = Integer.MAX_VALUE;
		int x = 0;
		int y = 0;
		
		for (int dy = -1; dy <= 1; dy++) {
			for (int dx = -1; dx <= 1; dx++) {
				int i = start.getY() + dy;
				int j = start.getX() + dx;
				if (i < 0 || i >= h || j < 0 || j >= w || (dx == 0 && dy == 0)) { continue; }
				int d = distances.get(i*w + j);
				if (d < min && (gc.isOccupiable(new MapLocation(start.getPlanet(), j, i)) != 0)) {
					x = dx;
					y = dy;
					min = d;
				}
			}
		}
		
		if (distances.get(start.getY()*w + start.getX()) < min) {
			return null;
		}
		
		if (y == -1) {
			if (x == -1) { return Direction.Southwest; }
			else if (x == 0) { return Direction.South; }
			else { return Direction.Southeast; }
		} else if (y == 0) {
			if (x == -1) { return Direction.West; }
			else { return Direction.East; }
		} else {
			if (x == -1) { return Direction.Northwest; }
			else if (x == 0) { return Direction.North; }
			else { return Direction.Northeast; }
		}
	}
}