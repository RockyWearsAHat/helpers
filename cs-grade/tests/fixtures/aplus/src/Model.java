package app;
import java.util.HashMap;
/** The model role. */
public interface Model { int size(); }
/** Default model backed by a HashMap and TreeMap. */
public class ModelImpl implements Model {
  private final HashMap<Integer,String> m = new HashMap<>();
  int size() { return m.size(); }
}
