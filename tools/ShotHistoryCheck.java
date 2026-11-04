import dev.jakmc.client.SnapshotHistory;
import java.util.Map;
public class ShotHistoryCheck {
 public static void main(String[]args){
  var history=new SnapshotHistory<Map<String,Double>>();
  history.accept(1,100,Map.of());history.accept(1,101,Map.of("shot",0.0));
  history.accept(1,102,Map.of("shot",10.0));history.accept(1,103,Map.of());
  if(!history.sample(100.99).before().value().isEmpty())throw new AssertionError("Shot appeared before its firing tick");
  var middle=history.sample(101.5);
  double at=middle.before().value().get("shot")+(middle.after().value().get("shot")-middle.before().value().get("shot"))*middle.fraction();
  if(Math.abs(at-5)>1e-6||Math.abs(middle.elapsed()-.025)>1e-6)throw new AssertionError("Body/shot render tick differs");
  if(!history.sample(102.99).before().value().containsKey("shot"))throw new AssertionError("Shot vanished before displayed impact");
  if(!history.sample(103).before().value().isEmpty())throw new AssertionError("Dead shot survived its displayed tick");
  history.accept(2,104,Map.of("new",50.0));
  if(history.sample(103).before().value().containsKey("shot"))throw new AssertionError("Cross-session ghost");
  for(int i=105;i<140;i++)history.accept(2,i,Map.of("new",(double)i));
  if(history.sample(0).before().tick()!=132)throw new AssertionError("Unbounded snapshot retention");
  var stalled=history.sample(150);
  if(stalled.before()!=stalled.after()||stalled.elapsed()!=0)throw new AssertionError("Stalled stream extrapolates through collision");
  System.out.println("PASS: shared tick interpolation, firing/impact lifetimes, session reset, bounded history and no extrapolation");
 }
}
