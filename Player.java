// import the API.
// See xxx for the javadocs.
import bc.*;
import navigation.Navigator;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.concurrent.ThreadLocalRandom;

public class Player {
    public static void main(String[] args) {

        // Connect to the manager, starting the game
        GameController gc = new GameController();
        
        // Navigation
        Navigator nav = new Navigator(gc.startingMap(gc.planet()));
        
        PlanetMap startingMap = gc.startingMap(gc.planet());
        AsteroidPattern asteroidPattern = gc.asteroidPattern(); 

        // Initialize Karb Locations
        HashMap<MapLocation, Integer> karbLocs = new HashMap<>();
        for(int x=0;x<startingMap.getWidth();x++) {
            for(int y=0;y<startingMap.getHeight();y++) {
                MapLocation loc = new MapLocation(gc.planet(),x,y);
                int karb = (int) startingMap.initialKarboniteAt(loc);
                if(karb != 0) {
                    karbLocs.put(loc, karb);
                }
            }
        }

        gc.queueResearch(UnitType.Worker);
        gc.queueResearch(UnitType.Rocket);

        MapLocation enStartLoc=null;
        VecUnit startingUnits = startingMap.getInitial_units();
        for(int i=0;i<startingUnits.size();i++) {
            if(startingUnits.get(i).team() != gc.team()) {
                enStartLoc = startingUnits.get(i).location().mapLocation();
                break;
            }
        }

        int numRockets = 0;

        while (true) {
            System.out.println("Time In Pool: " + gc.getTimeLeftMs());
            // Update karbLocs
            for(MapLocation loc:((HashMap<MapLocation,Integer>)karbLocs.clone()).keySet()) {
                if(gc.canSenseLocation(loc)) {
                    int karb = (int) gc.karboniteAt(loc);
                    if(karb != 0) {
                        karbLocs.replace(loc, karb);
                    }
                    else {
                        karbLocs.remove(loc);
                    }
                }
            }
            if(gc.planet().equals(Planet.Mars)) {
                try {
                    AsteroidStrike asteroid = asteroidPattern.asteroid(gc.round());
                    MapLocation loc = asteroid.getLocation();
                    int karb = (int) asteroid.getKarbonite();
                    if(karbLocs.containsKey(loc)) {
                        karb += karbLocs.get(loc);
                    }
                    karbLocs.put(loc, karb);
                }
                catch (Exception e){}
            }

            // Store Units
            VecUnit units = gc.myUnits();
            ArrayList<Unit> facts = getType(gc,UnitType.Factory);
            ArrayList<Unit> unfinishedFacts = new ArrayList<>();
            ArrayList<Unit> finishedFacts = new ArrayList<>();
            for(Unit f:facts) {
                if(f.structureIsBuilt()==1) {
                    finishedFacts.add(f);
                }
                else {
                    unfinishedFacts.add(f);
                }
            }
            ArrayList<Unit> rockets = getType(gc,UnitType.Rocket);
            ArrayList<Unit> finishedRockets = new ArrayList<>();
            ArrayList<Unit> unfinishedRockets = new ArrayList<>();
            for(Unit r:rockets){
                if(r.structureIsBuilt()==1) {
                    finishedRockets.add(r);
                }
                else {
                    unfinishedRockets.add(r);
                }
            }
            ArrayList<Unit> workers = getType(gc,UnitType.Worker);
            ArrayList<Unit> rangers = getType(gc,UnitType.Ranger);
            ArrayList<Unit> knights = getType(gc,UnitType.Knight);
            ArrayList<Unit> mages = getType(gc,UnitType.Mage);
            ArrayList<Unit> healers = getType(gc,UnitType.Healer);

            // Command Units
            for(Unit fact:finishedFacts) {
                if(workers.size()==0) {
                    tryBuild(gc,fact,UnitType.Worker);
                }
                else if (gc.researchInfo().getLevel(UnitType.Rocket) == 0 || rockets.size() != 0 ){
                    tryBuild(gc,fact,UnitType.Ranger);
                }
                tryUnload(gc,fact);
            }
            ArrayList<MapLocation> keys = new ArrayList<>(karbLocs.keySet());
            HashMap<Unit,MapLocation> workerTarget = new HashMap<>();
            HashSet<MapLocation> karbAssigned = new HashSet<>();
            HashMap<Unit,Integer> rocketAssigned = new HashMap<>();

            for(Unit r:finishedRockets) {
                PlanetMap mars = gc.startingMap(Planet.Mars);
                int randX = ThreadLocalRandom.current().nextInt(0, (int)mars.getWidth()+ 1);
                int randY = ThreadLocalRandom.current().nextInt(0, (int)mars.getHeight()+1);
                if(r.structureGarrison().size() >=8 && gc.canLaunchRocket(r.id(),new MapLocation(Planet.Mars,randX,randY))) {
                    gc.launchRocket(r.id(),new MapLocation(Planet.Mars,randX,randY));
                    System.out.println("LAUNCH!");
                }

                if(r.rocketIsUsed()==1) {
                    for(int i=0;i<r.structureGarrison().size();i++) {
                        tryUnload(gc,r);
                    }
                }
                else {
                    VecUnit nearbyUnits = gc.senseNearbyUnits(r.location().mapLocation(),2);
                    for(int i=0;i<nearbyUnits.size();i++) {
                        if(gc.canLoad(r.id(),nearbyUnits.get(i).id())) {
                            gc.load(r.id(),nearbyUnits.get(i).id());
                        }
                    }
                }
            }

            for(Unit worker:workers) {
                if(!worker.location().isOnMap()){
                    continue;
                }
                if(tryRockets(gc,worker, finishedRockets,rocketAssigned,nav)) {

                }
                else if(((facts.size()!=0 && workers.size() < 10) || (worker.location().mapLocation().getPlanet() == Planet.Mars && workers.size() < 10)) && tryReplicate(gc,worker)) {

                }
                if(tryBuild(gc,worker)){
                }
                else if(facts.size() < 4 && tryBlueprint(gc,worker,UnitType.Factory)){
                }
                else if(tryBlueprint(gc,worker,UnitType.Rocket)) {

                }
                else if(tryHarvest(gc,worker)) {

                }
                else if(unfinishedFacts.size()!=0 && moveTo(gc,nav,worker,unfinishedFacts.get(0).location().mapLocation())) {
                    
                }
                else if(unfinishedRockets.size()!=0 && moveTo(gc,nav,worker,unfinishedRockets.get(0).location().mapLocation())) {

                }
                
                else if (karbLocs.size() != 0){
                    if(workerTarget.containsKey(worker)) {
                        if(workerTarget.get(worker).equals(worker.location().mapLocation())) {
                            workerTarget.remove(worker);
                        }
                        else {
                            moveTo(gc,nav,worker,workerTarget.get(worker));
                        }
                    }
                    else {
                        Collections.sort(keys, (MapLocation loc1, MapLocation loc2) -> nav.between(worker.location().mapLocation(),loc1) - nav.between(worker.location().mapLocation(),loc2));
                        for(MapLocation loc:keys) {
                            if(!karbAssigned.contains(loc)) {
                                workerTarget.put(worker,loc);
                                karbAssigned.add(loc);
                                moveTo(gc,nav,worker,loc);
                                break;
                            }
                        }

                    }

                }
            }

            for(Unit ranger:rangers) {
                if(!ranger.location().isOnMap()){
                    continue;
                }

                if(tryAttack(gc,ranger)) {

                }
                else if(tryRockets(gc,ranger, finishedRockets,rocketAssigned,nav)) {

                }
                else if (gc.planet().equals(Planet.Earth) && moveTo(gc, nav, ranger,enStartLoc)){
                    
                }
                else if (randMove(gc,ranger)){

                }
            }

            if (gc.round()%10 == 0) {
                System.runFinalization();
                System.gc();
            }
            gc.nextTurn();
        }
    }

    public static boolean tryRockets(GameController gc, Unit unit, ArrayList<Unit> rockets, HashMap<Unit,Integer> rocketAssigned, Navigator nav) {
        for(Unit r: rockets) {
            if(r.rocketIsUsed() == 1) {
                return false;
            }
            if(!rocketAssigned.containsKey(r) || rocketAssigned.get(r) <8) {
                if(gc.canLoad(r.id(),unit.id())) {
                    gc.load(r.id(),unit.id());
                }
                else {
                    moveTo(gc, nav, unit,r.location().mapLocation());
                }
                int numAssigned = 1;
                if(rocketAssigned.containsKey(r)) {
                    numAssigned += rocketAssigned.get(r);
                }
                rocketAssigned.put(r,numAssigned);
                return true;
            }
        }
        return false;
    }

    public static ArrayList<Unit> getType(GameController gc,UnitType type) {
        VecUnit units = gc.myUnits();
        ArrayList<Unit> out = new ArrayList<Unit>();
        for(int i=0;i<units.size();i++) {
            Unit unit = units.get(i);
            if (unit.unitType().equals(type)) {
                out.add(unit);
            }
        }
        return out;
    }

    public static boolean randMove(GameController gc,Unit unit) {
        ArrayList<Direction> dirs = new ArrayList<Direction>();
        for (Direction d: Direction.values()) {
            dirs.add(d);
        }
        Collections.shuffle(dirs);

        for(Direction d: dirs) {
            if (gc.isMoveReady(unit.id()) && gc.canMove(unit.id(),d)) {
                gc.moveRobot(unit.id(),d);
                return true;
            }
        }
        return false;
    }
    
    public static boolean moveTo(GameController gc, Navigator nav, Unit unit, MapLocation loc) {
    	Direction d = nav.navigate(gc, unit.location().mapLocation(), loc);
        if (gc.isMoveReady(unit.id()) && d != null && gc.canMove(unit.id(),d)) {
            gc.moveRobot(unit.id(),d);
            return true;
        }
        return false;
    }

    public static boolean tryBuild(GameController gc,Unit fact, UnitType type) {
        if(gc.canProduceRobot(fact.id(),type)) {
            gc.produceRobot(fact.id(),type);
            return true;
        }
        return false;
    }

    public static void tryUnload(GameController gc, Unit fact) {
        Direction[] dirs = Direction.values();
        for(int i=0;i<dirs.length && fact.structureGarrison().size() > 0;i++) {
            if(gc.canUnload(fact.id(), dirs[i])) {
                gc.unload(fact.id(), dirs[i]);
            }
        }
    }

    public static boolean tryBlueprint(GameController gc, Unit worker, UnitType type) {
        Direction[] dirs = Direction.values();
        for(int i=0;i<dirs.length;i++) {
            if(gc.canBlueprint(worker.id(), type, dirs[i])) {
                gc.blueprint(worker.id(),type,dirs[i]);
                return true;
            }
        }
        return false;
    }

    public static boolean tryHarvest(GameController gc, Unit worker) {
        Direction[] dirs = Direction.values();
        for(int i=0;i<dirs.length;i++) {
            if(gc.canHarvest(worker.id(),dirs[i])) {
                gc.harvest(worker.id(),dirs[i]);
                return true;
            }
        }
        return false;
    }

    public static boolean tryBuild(GameController gc, Unit worker) {
        VecUnit units = gc.senseNearbyUnits(worker.location().mapLocation(),2);
        for(int i=0;i<units.size();i++){
            Unit unit = units.get(i);
            if(gc.canBuild(worker.id(), unit.id())) {
                gc.build(worker.id(),unit.id());
                return true;
            }
        }
        return false;
    }

    public static boolean tryAttack(GameController gc, Unit unit) {
        VecUnit units = gc.senseNearbyUnits(unit.location().mapLocation(),unit.attackRange());
        for(int i=0;i<units.size();i++) {
            Unit target = units.get(i);
            if(!target.team().equals(gc.team()) && unit.attackHeat()<10 && gc.canAttack(unit.id(),target.id())) {
                gc.attack(unit.id(),target.id());
                return true;
            }
        }
        return false;
    }

    public static boolean tryReplicate(GameController gc, Unit worker){
        Direction[] dirs = Direction.values();
        for(int i=0;i<dirs.length;i++){
            if(gc.canReplicate(worker.id(),dirs[i])){
                gc.replicate(worker.id(),dirs[i]);
                return true;
            }
        }
        return false;
    }
}
