package navigation;

import java.util.ArrayList;
import java.util.HashMap;
import bc.*;

class Point {
	public int x;
	public int y;
	
	public Point(int x, int y) {
		this.x = x;
		this.y = y;
	}
	
	public boolean equals(Object other) {
		if (!(other instanceof Point)) { return false; }
		else {
			Point p = (Point) other;
			return p.x == this.x && p.y == this.y;
		}
	}
	
	public int hashCode() {
		return y*x;
	}
}

public class Navigator {
	
	private int w;
	private int h;
	private ArrayList<ArrayList<Point>> terrain;
	private HashMap<Point, Route> cache;

	public Navigator(PlanetMap map) {
		w = (int) map.getWidth();
		h = (int) map.getHeight();
		Planet planet = map.getPlanet();
		boolean[] passable = new boolean[w*h];
		terrain = new ArrayList<>(w*h); 
		cache = new HashMap<>();
		
		// Mark which tiles are passable
		for (int y = 0; y < h; y++) {
			for (int x = 0; x < w; x++) {
				passable[y*w + x] = map.isPassableTerrainAt(new MapLocation(planet, x, y)) != 0;
				terrain.add(new ArrayList<>());
			}
		}
		
		// Compute each tile's neighbors
		for (int y = 0; y < h; y++) {
			for (int x = 0; x < w; x++) {
				ArrayList<Point> adj = terrain.get(y*w + x);
				for (int dy = -1; dy <= 1; dy++) {
					for (int dx = -1; dx <= 1; dx++) {
						int i = y + dy;
						int j = x + dx;
						if (i < 0 || i >= h || j < 0 || j >= w || (dx == 0 && dy == 0)) { 
							continue;
						} else if (passable[i*w + j]){
							adj.add(new Point(j, i));
						}
					}
				}
			}
		}
	}
	
	public Direction navigate(GameController gc, MapLocation from, MapLocation to) {
		Point key = new Point(to.getX(), to.getY());
		if (!cache.containsKey(key)) {
			cache.put(key, new Route(terrain, w, h, to));
		}
		return cache.get(key).from(gc, from);
	}
	
	public int between(MapLocation from, MapLocation to) {
		Point key = new Point(to.getX(), to.getY());
		if (!cache.containsKey(key)) {
			cache.put(key,  new Route(terrain, w, h, to));
		}
		return cache.get(key).between(from);
	}
}