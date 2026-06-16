package sample.model;
import java.util.HashMap;
import java.util.ArrayList;
/** Default board. */
public final class BoardImpl implements Board {
  private final HashMap<Integer,String> cells = new HashMap<>();
  /** Make board. */
  public BoardImpl() {}
  /** @return size */
  public int size() { return cells.size(); }
  // public int debug() { return 0; }
  public void noisy() { System.out.println("debug"); }
}
