// import the API.
// See xxx for the javadocs.
import bc.*;
import navigation.Navigator;

import java.util.ArrayList;
import java.util.Collections;

public class Player {
    public static void main(String[] args) {

        // Connect to the manager, starting the game
        GameController gc = new GameController();
        
        // Navigation
        Navigator nav = new Navigator(gc.startingMap(gc.planet()));
        
        // Direction is a normal java enum.
        Direction[] directions = Direction.values();

        while (true) {
            VecUnit units = gc.myUnits();
            ArrayList<Unit> facts = getType(gc,UnitType.Factory);
            ArrayList<Unit> workers = getType(gc,UnitType.Worker);
            ArrayList<Unit> rangers = getType(gc,UnitType.Ranger);
            ArrayList<Unit> knights = getType(gc,UnitType.Knight);
            ArrayList<Unit> mages = getType(gc,UnitType.Mage);
            ArrayList<Unit> healers = getType(gc,UnitType.Healer);

            for(Unit fact:facts) {
                tryBuild(gc,fact,UnitType.Ranger);
                tryUnload(gc,fact);
            }

            for(Unit worker:workers) {
//                if(facts.size()!=0 && tryReplicate(gc,worker)) {

//                }
                if(tryBuild(gc,worker)){
                }
                else if(tryBlueprint(gc,worker,UnitType.Factory)){

                }
                else if(tryHarvest(gc,worker)) {
                }
                else if(randMove(gc,worker)) {
                }

            }

            for(Unit ranger:rangers) {
                if(!ranger.location().isOnMap()){
                    continue;
                }
                moveOrigin(gc, nav, ranger);
//                if(tryAttack(gc,ranger)) {
//
//                }
            }
            gc.nextTurn();
        }
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
    
    public static boolean moveOrigin(GameController gc, Navigator nav, Unit unit) {
    	Direction d = nav.navigate(gc, unit.location().mapLocation(), new MapLocation(gc.planet(), 0, 0));
        if (gc.isMoveReady(unit.id()) && d != null) {
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
