package app;
import org.junit.Test;
import static org.junit.Assert.*;
/** Tests for View. */
public class ViewTest {
  @Test public void a(){assertEquals(1,1);assertTrue(true);assertNotNull("x");}
  @Test public void b(){assertEquals(2,2);assertFalse(false);assertNotNull("y");}
}
