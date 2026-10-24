import javax.imageio.*; import java.awt.image.*; import java.io.*;
public class MkBase { public static void main(String[] a) throws Exception {
 int w=Integer.parseInt(a[1]), h=Integer.parseInt(a[2]);
 BufferedImage im=new BufferedImage(w,h,BufferedImage.TYPE_INT_RGB);
 for(int y=0;y<h;y++)for(int x=0;x<w;x++)im.setRGB(x,y,((x*255/w)<<16)|((y*255/h)<<8)|128);
 ImageIO.write(im,"jpg",new File(a[0])); } }
